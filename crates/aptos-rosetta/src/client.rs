// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::EmptyRequest,
    types::{
        AccountBalanceRequest, AccountBalanceResponse, NetworkListResponse, NetworkOptionsResponse,
        NetworkRequest, NetworkStatusResponse,
    },
};
use aptos_rest_client::aptos_api_types::mime_types::JSON;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use reqwest::{header::CONTENT_TYPE, Client as ReqwestClient};
use url::Url;

pub struct RosettaClient {
    address: Url,
    inner: ReqwestClient,
}

impl RosettaClient {
    pub fn new(address: Url) -> RosettaClient {
        RosettaClient {
            address,
            inner: ReqwestClient::new(),
        }
    }

    pub async fn account_balance_simple(
        &self,
        account: AccountAddress,
        chain_id: ChainId,
    ) -> anyhow::Result<AccountBalanceResponse> {
        let request = AccountBalanceRequest {
            network_identifier: chain_id.into(),
            account_identifier: account.into(),
            block_identifier: None,
            currencies: None,
        };
        self.account_balance(&request).await
    }

    pub async fn account_balance(
        &self,
        request: &AccountBalanceRequest,
    ) -> anyhow::Result<AccountBalanceResponse> {
        let response = self
            .inner
            .post(self.address.join("account/balance").unwrap())
            .header(CONTENT_TYPE, JSON)
            .body(serde_json::to_string(request)?)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn network_list(&self) -> anyhow::Result<NetworkListResponse> {
        let response = self
            .inner
            .post(self.address.join("network/list").unwrap())
            .header(CONTENT_TYPE, JSON)
            .body(serde_json::to_string(&EmptyRequest)?)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn network_options(
        &self,
        request: &NetworkRequest,
    ) -> anyhow::Result<NetworkOptionsResponse> {
        let response = self
            .inner
            .post(self.address.join("network/options").unwrap())
            .header(CONTENT_TYPE, JSON)
            .body(serde_json::to_string(request)?)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn network_status(
        &self,
        request: &NetworkRequest,
    ) -> anyhow::Result<NetworkStatusResponse> {
        let response = self
            .inner
            .post(self.address.join("network/status").unwrap())
            .header(CONTENT_TYPE, JSON)
            .body(serde_json::to_string(request)?)
            .send()
            .await?;

        self.json(response).await
    }

    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<T> {
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Request failed: {:?}",
                response.error_for_status()
            ));
        }

        Ok(response.json().await?)
    }
}
