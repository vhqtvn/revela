// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_network::protocols::network::RpcError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Aptos network rpc error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Subscription termination: {0}")]
    SubscriptionTermination(String),

    #[error("Unexpected error encountered: {0}")]
    UnexpectedError(String),
}

impl Error {
    /// Returns a summary label for the error
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::NetworkError(_) => "network_error",
            Self::RpcError(_) => "rpc_error",
            Self::SubscriptionTermination(_) => "subscription_termination",
            Self::UnexpectedError(_) => "unexpected_error",
        }
    }
}

impl From<aptos_network::application::error::Error> for Error {
    fn from(error: aptos_network::application::error::Error) -> Self {
        Error::NetworkError(error.to_string())
    }
}
