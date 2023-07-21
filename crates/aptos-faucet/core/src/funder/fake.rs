// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::FunderTrait;
use crate::endpoints::AptosTapError;
use aptos_sdk::types::{account_address::AccountAddress, transaction::SignedTransaction};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FakeFunderConfig {}

pub struct FakeFunder;

#[async_trait]
impl FunderTrait for FakeFunder {
    async fn fund(
        &self,
        _amount: Option<u64>,
        _receiver_address: AccountAddress,
        _check_only: bool,
        _did_bypass_checkers: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError> {
        Ok(vec![])
    }

    fn get_amount(&self, amount: Option<u64>, _did_bypass_checkers: bool) -> u64 {
        amount.unwrap_or(100)
    }
}
