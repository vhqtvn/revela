// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliError, CliTypedResult};
use anyhow::Context;
use aptos_protos::indexer::v1::GetTransactionsRequest;
use futures::StreamExt;
use reqwest::Url;
use serde::Serialize;
use std::time::Duration;
use tokio::time::Instant;

const MAX_WAIT_S: u64 = 35;
const WAIT_INTERVAL_MS: u64 = 150;

/// This provides a single place to define a variety of different healthchecks.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum HealthChecker {
    /// Check that an HTTP API is up. The second param is the name of the HTTP service.
    Http(Url, String),
    /// Check that the node API is up. This is just a specific case of Http for extra
    /// guarantees around liveliness.
    NodeApi(Url),
    /// Check that a data service GRPC stream is up.
    DataServiceGrpc(Url),
}

impl HealthChecker {
    pub async fn check(&self) -> CliTypedResult<()> {
        match self {
            HealthChecker::Http(url, _) => {
                reqwest::get(Url::clone(url))
                    .await
                    .with_context(|| format!("Failed to GET {}", url))?;
                Ok(())
            },
            HealthChecker::NodeApi(url) => {
                aptos_rest_client::Client::new(Url::clone(url))
                    .get_index()
                    .await?;
                Ok(())
            },
            HealthChecker::DataServiceGrpc(url) => {
                let mut client = aptos_indexer_grpc_utils::create_data_service_grpc_client(
                    url.clone(),
                    Some(Duration::from_secs(5)),
                )
                .await;
                let request = tonic::Request::new(GetTransactionsRequest {
                    starting_version: Some(0),
                    ..Default::default()
                });
                // Make sure we can stream the first message from the stream.
                client
                    .get_transactions(request)
                    .await
                    .map_err(|err| {
                        CliError::UnexpectedError(format!("GRPC connection error: {:#}", err))
                    })?
                    .into_inner()
                    .next()
                    .await
                    .context("Did not receive init signal from data service GRPC stream")?
                    .map_err(|err| {
                        CliError::UnexpectedError(format!(
                            "Error processing first message from GRPC stream: {:#}",
                            err
                        ))
                    })?;
                Ok(())
            },
        }
    }

    /// Wait up to MAX_WAIT_S seconds for a service to start up.
    pub async fn wait(
        &self,
        // The service, if any, waiting for this service to start up.
        waiting_service: Option<&str>,
    ) -> CliTypedResult<()> {
        let prefix = self.to_string();
        wait_for_startup(|| self.check(), match waiting_service {
            Some(waiting_service) => {
                format!(
                    "{} at {} did not start up before {}",
                    prefix,
                    waiting_service,
                    self.address_str()
                )
            },
            None => format!("{} at {} did not start up", prefix, self.address_str()),
        })
        .await
    }

    pub fn address_str(&self) -> &str {
        match self {
            HealthChecker::Http(url, _) => url.as_str(),
            HealthChecker::NodeApi(url) => url.as_str(),
            HealthChecker::DataServiceGrpc(url) => url.as_str(),
        }
    }

    /// Given a port, make an instance of HealthChecker::Http targeting 127.0.0.1.
    pub fn http_checker_from_port(port: u16, name: String) -> Self {
        Self::Http(
            Url::parse(&format!("http://127.0.0.1:{}", port,)).unwrap(),
            name,
        )
    }
}

impl std::fmt::Display for HealthChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthChecker::Http(_, name) => write!(f, "{}", name),
            HealthChecker::NodeApi(_) => write!(f, "Node API"),
            HealthChecker::DataServiceGrpc(_) => write!(f, "Transaction stream"),
        }
    }
}

async fn wait_for_startup<F, Fut>(check_fn: F, error_message: String) -> CliTypedResult<()>
where
    F: Fn() -> Fut,
    Fut: futures::Future<Output = CliTypedResult<()>>,
{
    let max_wait = Duration::from_secs(MAX_WAIT_S);
    let wait_interval = Duration::from_millis(WAIT_INTERVAL_MS);

    let start = Instant::now();
    let mut started_successfully = false;

    let mut last_error_message = None;
    while start.elapsed() < max_wait {
        match check_fn().await {
            Ok(_) => {
                started_successfully = true;
                break;
            },
            Err(err) => {
                last_error_message = Some(format!("{:#}", err));
            },
        }
        tokio::time::sleep(wait_interval).await
    }

    if !started_successfully {
        let error_message = match last_error_message {
            Some(last_error_message) => format!("{}: {}", error_message, last_error_message),
            None => error_message,
        };
        return Err(CliError::UnexpectedError(error_message));
    }

    Ok(())
}
