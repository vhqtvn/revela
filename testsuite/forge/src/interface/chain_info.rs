// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosPublicInfo, Coffer, NFTPublicInfo, PublicInfo, Result};
use diem_rest_client::Client as RestClient;
use diem_sdk::{
    transaction_builder::{Currency, TransactionFactory},
    types::{
        account_address::AccountAddress, chain_id::ChainId,
        transaction::authenticator::AuthenticationKey, LocalAccount,
    },
};
use reqwest::Url;

#[derive(Debug)]
pub struct ChainInfo<'t> {
    pub root_account: &'t mut LocalAccount,
    pub treasury_compliance_account: &'t mut LocalAccount,
    pub designated_dealer_account: &'t mut LocalAccount,
    pub rest_api_url: String,
    pub chain_id: ChainId,
}

impl<'t> ChainInfo<'t> {
    pub fn new(
        root_account: &'t mut LocalAccount,
        treasury_compliance_account: &'t mut LocalAccount,
        designated_dealer_account: &'t mut LocalAccount,
        rest_api_url: String,
        chain_id: ChainId,
    ) -> Self {
        Self {
            root_account,
            treasury_compliance_account,
            designated_dealer_account,
            rest_api_url,
            chain_id,
        }
    }

    pub fn designated_dealer_account(&mut self) -> &mut LocalAccount {
        self.designated_dealer_account
    }

    pub fn root_account(&mut self) -> &mut LocalAccount {
        self.root_account
    }

    pub fn treasury_compliance_account(&mut self) -> &mut LocalAccount {
        self.treasury_compliance_account
    }

    pub fn rest_api(&self) -> &str {
        &self.rest_api_url
    }

    pub fn rest_client(&self) -> RestClient {
        RestClient::new(Url::parse(self.rest_api()).unwrap())
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        TransactionFactory::new(self.chain_id())
    }

    pub async fn create_parent_vasp_account(
        &mut self,
        currency: Currency,
        authentication_key: AuthenticationKey,
    ) -> Result<()> {
        let factory = self.transaction_factory();
        let client = self.rest_client();
        let treasury_compliance_account = self.treasury_compliance_account();

        let create_account_txn = treasury_compliance_account.sign_with_transaction_builder(
            factory.create_parent_vasp_account(
                currency,
                0,
                authentication_key,
                &format!("No. {} VASP", treasury_compliance_account.sequence_number()),
                false,
            ),
        );
        client.submit_and_wait(&create_account_txn).await?;
        Ok(())
    }

    pub async fn create_designated_dealer_account(
        &mut self,
        currency: Currency,
        authentication_key: AuthenticationKey,
    ) -> Result<()> {
        let factory = self.transaction_factory();
        let client = self.rest_client();
        let treasury_compliance_account = self.treasury_compliance_account();

        let create_account_txn = treasury_compliance_account.sign_with_transaction_builder(
            factory.create_designated_dealer(
                currency,
                0, // sliding_nonce
                authentication_key,
                &format!("No. {} DD", treasury_compliance_account.sequence_number()),
                false, // add all currencies
            ),
        );
        client.submit_and_wait(&create_account_txn).await?;
        Ok(())
    }

    pub async fn fund(
        &mut self,
        currency: Currency,
        address: AccountAddress,
        amount: u64,
    ) -> Result<()> {
        let factory = self.transaction_factory();
        let client = self.rest_client();
        let designated_dealer_account = self.designated_dealer_account();
        let fund_account_txn = designated_dealer_account
            .sign_with_transaction_builder(factory.peer_to_peer(currency, address, amount));
        client.submit_and_wait(&fund_account_txn).await?;
        Ok(())
    }

    pub fn into_public_info(self) -> PublicInfo<'t> {
        PublicInfo::new(
            self.chain_id,
            Coffer::TreasuryCompliance {
                transaction_factory: TransactionFactory::new(self.chain_id),
                rest_client: self.rest_client(),
                treasury_compliance_account: self.treasury_compliance_account,
                designated_dealer_account: self.designated_dealer_account,
            },
            self.rest_api_url.clone(),
        )
    }

    pub fn into_nft_public_info(self) -> NFTPublicInfo<'t> {
        NFTPublicInfo::new(self.chain_id, self.rest_api_url.clone(), self.root_account)
    }

    pub fn into_aptos_public_info(self) -> AptosPublicInfo<'t> {
        AptosPublicInfo::new(self.chain_id, self.rest_api_url.clone(), self.root_account)
    }
}
