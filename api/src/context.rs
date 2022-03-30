// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::{Error, LedgerInfo, MoveConverter, TransactionOnChainData};
use aptos_config::config::{ApiConfig, RoleType};
use aptos_crypto::HashValue;
use aptos_mempool::{MempoolClientRequest, MempoolClientSender, SubmissionStatus};
use aptos_types::{
    account_address::AccountAddress,
    account_state::AccountState,
    account_state_blob::AccountStateBlob,
    chain_id::ChainId,
    contract_event::ContractEvent,
    event::EventKey,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{SignedTransaction, TransactionWithProof},
};
use storage_interface::{MoveDbReader, Order};

use anyhow::{ensure, format_err, Result};
use aptos_types::state_store::state_key::StateKey;
use futures::{channel::oneshot, SinkExt};
use std::{
    borrow::Borrow,
    convert::{Infallible, TryFrom},
    sync::Arc,
};
use warp::{filters::BoxedFilter, Filter, Reply};

// Context holds application scope context
#[derive(Clone)]
pub struct Context {
    chain_id: ChainId,
    db: Arc<dyn MoveDbReader>,
    mp_sender: MempoolClientSender,
    role: RoleType,
    api_config: ApiConfig,
}

impl Context {
    pub fn new(
        chain_id: ChainId,
        db: Arc<dyn MoveDbReader>,
        mp_sender: MempoolClientSender,
        role: RoleType,
        api_config: ApiConfig,
    ) -> Self {
        Self {
            chain_id,
            db,
            mp_sender,
            role,
            api_config,
        }
    }

    pub fn move_converter(&self) -> MoveConverter<dyn MoveDbReader + '_> {
        MoveConverter::new(self.db.borrow())
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn content_length_limit(&self) -> u64 {
        self.api_config.content_length_limit()
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub async fn submit_transaction(&self, txn: SignedTransaction) -> Result<SubmissionStatus> {
        let (req_sender, callback) = oneshot::channel();
        self.mp_sender
            .clone()
            .send(MempoolClientRequest::SubmitTransaction(txn, req_sender))
            .await?;

        callback.await?
    }

    pub fn get_latest_ledger_info(&self) -> Result<LedgerInfo, Error> {
        Ok(LedgerInfo::new(
            &self.chain_id(),
            &self.get_latest_ledger_info_with_signatures()?,
        ))
    }

    pub fn get_latest_ledger_info_with_signatures(&self) -> Result<LedgerInfoWithSignatures> {
        self.db.get_latest_ledger_info()
    }

    pub fn get_account_state(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<Option<AccountState>> {
        let state = self.get_account_state_blob(address, version)?;
        Ok(match state {
            Some(blob) => Some(AccountState::try_from(&blob)?),
            None => None,
        })
    }

    pub fn get_account_state_blob(
        &self,
        account: AccountAddress,
        version: u64,
    ) -> Result<Option<AccountStateBlob>> {
        let (state_value, _) = self.db.get_state_value_with_proof_by_version(
            &StateKey::AccountAddressKey(account),
            version,
        )?;
        Ok(state_value.map(AccountStateBlob::from))
    }

    pub fn get_block_timestamp(&self, version: u64) -> Result<u64> {
        self.db.get_block_timestamp(version)
    }

    pub fn get_transactions(
        &self,
        start_version: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<TransactionOnChainData>> {
        let data = self
            .db
            .get_transactions(start_version, limit as u64, ledger_version, true)?;

        let txn_start_version = data
            .first_transaction_version
            .ok_or_else(|| format_err!("no start version from database"))?;
        ensure!(
            txn_start_version == start_version,
            "invalid start version from database: {} != {}",
            txn_start_version,
            start_version
        );

        let txns = data.transactions;
        let infos = data.proof.transaction_infos;
        let events = data.events.unwrap_or_default();

        ensure!(
            txns.len() == infos.len() && txns.len() == events.len(),
            "invalid data size from database: {}, {}, {}",
            txns.len(),
            infos.len(),
            events.len()
        );

        txns.into_iter()
            .zip(infos.into_iter())
            .zip(events.into_iter())
            .enumerate()
            .map(|(i, ((txn, info), events))| {
                let version = start_version + i as u64;
                self.get_accumulator_root_hash(version)
                    .map(|h| (version, txn, info, events, h).into())
            })
            .collect()
    }

    pub fn get_account_transactions(
        &self,
        address: AccountAddress,
        start_seq_number: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<TransactionOnChainData>> {
        let txns = self.db.get_account_transactions(
            address,
            start_seq_number,
            limit as u64,
            true,
            ledger_version,
        )?;
        txns.into_inner()
            .into_iter()
            .map(|t| self.convert_into_transaction_on_chain_data(t))
            .collect::<Result<Vec<_>>>()
    }

    pub fn get_transaction_by_hash(
        &self,
        hash: HashValue,
        ledger_version: u64,
    ) -> Result<Option<TransactionOnChainData>> {
        self.db
            .get_transaction_by_hash(hash, ledger_version, true)?
            .map(|t| self.convert_into_transaction_on_chain_data(t))
            .transpose()
    }

    pub async fn get_pending_transaction_by_hash(
        &self,
        hash: HashValue,
    ) -> Result<Option<SignedTransaction>> {
        let (req_sender, callback) = oneshot::channel();

        self.mp_sender
            .clone()
            .send(MempoolClientRequest::GetTransactionByHash(hash, req_sender))
            .await
            .map_err(anyhow::Error::from)?;

        callback.await.map_err(anyhow::Error::from)
    }

    pub fn get_transaction_by_version(
        &self,
        version: u64,
        ledger_version: u64,
    ) -> Result<TransactionOnChainData> {
        self.convert_into_transaction_on_chain_data(self.db.get_transaction_by_version(
            version,
            ledger_version,
            true,
        )?)
    }

    pub fn get_accumulator_root_hash(&self, version: u64) -> Result<HashValue> {
        self.db.get_accumulator_root_hash(version)
    }

    fn convert_into_transaction_on_chain_data(
        &self,
        txn: TransactionWithProof,
    ) -> Result<TransactionOnChainData> {
        self.get_accumulator_root_hash(txn.version)
            .map(|h| (txn, h).into())
    }

    pub fn get_events(
        &self,
        event_key: &EventKey,
        start: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<Vec<ContractEvent>> {
        let events = self
            .db
            .get_events(event_key, start, Order::Ascending, limit as u64)?;
        Ok(events
            .into_iter()
            .filter(|(version, _event)| version <= &ledger_version)
            .map(|(_, event)| event)
            .collect::<Vec<_>>())
    }

    pub fn health_check_route(&self) -> BoxedFilter<(impl Reply,)> {
        super::health_check::health_check_route(self.db.clone())
    }
}
