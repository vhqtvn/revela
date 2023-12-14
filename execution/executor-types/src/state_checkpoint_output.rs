// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::parsed_transaction_output::TransactionsWithParsedOutput;
use anyhow::{ensure, Result};
use aptos_crypto::HashValue;
use aptos_storage_interface::cached_state_view::ShardedStateCache;
use aptos_types::{state_store::ShardedStateUpdates, transaction::TransactionStatus};
use itertools::zip_eq;

#[derive(Default)]
pub struct TransactionsByStatus {
    // Statuses of the input transactions, in the same order as the input transactions.
    // Contains BlockMetadata/Validator transactions,
    // but doesn't contain StateCheckpoint/BlockEpilogue, as those get added during execution
    statuses_for_input_txns: Vec<TransactionStatus>,
    // List of all transactions to be committed, including StateCheckpoint/BlockEpilogue if needed.
    to_commit: TransactionsWithParsedOutput,
    to_discard: TransactionsWithParsedOutput,
    to_retry: TransactionsWithParsedOutput,
}

impl TransactionsByStatus {
    pub fn new(
        statuses_for_input_txns: Vec<TransactionStatus>,
        to_commit: TransactionsWithParsedOutput,
        to_discard: TransactionsWithParsedOutput,
        to_retry: TransactionsWithParsedOutput,
    ) -> Self {
        Self {
            statuses_for_input_txns,
            to_commit,
            to_discard,
            to_retry,
        }
    }

    pub fn input_txns_len(&self) -> usize {
        self.statuses_for_input_txns.len()
    }

    pub fn into_inner(
        self,
    ) -> (
        Vec<TransactionStatus>,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
    ) {
        (
            self.statuses_for_input_txns,
            self.to_commit,
            self.to_discard,
            self.to_retry,
        )
    }
}

#[derive(Default)]
pub struct StateCheckpointOutput {
    txns: TransactionsByStatus,
    per_version_state_updates: Vec<ShardedStateUpdates>,
    state_checkpoint_hashes: Vec<Option<HashValue>>,
    state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
    sharded_state_cache: ShardedStateCache,
}

impl StateCheckpointOutput {
    pub fn new(
        txns: TransactionsByStatus,
        per_version_state_updates: Vec<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        state_updates_before_last_checkpoint: Option<ShardedStateUpdates>,
        sharded_state_cache: ShardedStateCache,
    ) -> Self {
        Self {
            txns,
            per_version_state_updates,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
        }
    }

    pub fn input_txns_len(&self) -> usize {
        self.txns.input_txns_len()
    }

    pub fn txns_to_commit_len(&self) -> usize {
        self.txns.to_commit.len()
    }

    pub fn into_inner(
        self,
    ) -> (
        TransactionsByStatus,
        Vec<ShardedStateUpdates>,
        Vec<Option<HashValue>>,
        Option<ShardedStateUpdates>,
        ShardedStateCache,
    ) {
        (
            self.txns,
            self.per_version_state_updates,
            self.state_checkpoint_hashes,
            self.state_updates_before_last_checkpoint,
            self.sharded_state_cache,
        )
    }

    pub fn check_and_update_state_checkpoint_hashes(
        &mut self,
        trusted_hashes: Vec<Option<HashValue>>,
    ) -> Result<()> {
        let len = self.state_checkpoint_hashes.len();
        ensure!(
            len == trusted_hashes.len(),
            "Number of txns doesn't match. self: {len}, trusted: {}",
            trusted_hashes.len()
        );

        zip_eq(
            self.state_checkpoint_hashes.iter_mut(),
            trusted_hashes.iter(),
        )
        .try_for_each(|(self_hash, trusted_hash)| {
            if self_hash.is_none() && trusted_hash.is_some() {
                *self_hash = *trusted_hash;
            } else {
                ensure!(self_hash == trusted_hash,
                        "State checkpoint hash doesn't match, self: {self_hash:?}, trusted: {trusted_hash:?}");
            }
            Ok(())
        })
    }
}
