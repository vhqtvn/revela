// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{
        chunk_output::ChunkOutput, in_memory_state_calculator_v2::InMemoryStateCalculatorV2,
    },
    metrics::{APTOS_EXECUTOR_ERRORS, APTOS_EXECUTOR_OTHER_TIMERS_SECONDS},
};
use anyhow::{ensure, Result};
use aptos_crypto::{
    hash::{CryptoHash, EventAccumulatorHasher},
    HashValue,
};
use aptos_executor_types::{
    in_memory_state_calculator::InMemoryStateCalculator, ExecutedBlock, ExecutedChunk,
    ParsedTransactionOutput, TransactionData,
};
use aptos_logger::error;
use aptos_storage_interface::ExecutedTrees;
use aptos_types::{
    contract_event::ContractEvent,
    proof::accumulator::InMemoryAccumulator,
    state_store::{state_key::StateKey, state_value::StateValue, ShardedStateUpdates},
    transaction::{
        Transaction, TransactionInfo, TransactionOutput, TransactionStatus, TransactionToCommit,
    },
};
use rayon::prelude::*;
use std::{collections::HashMap, iter::repeat, sync::Arc};

pub struct ApplyChunkOutput;

impl ApplyChunkOutput {
    pub fn apply_block(
        chunk_output: ChunkOutput,
        base_view: &ExecutedTrees,
    ) -> Result<(ExecutedBlock, Vec<Transaction>, Vec<Transaction>)> {
        let ChunkOutput {
            state_cache,
            transactions,
            transaction_outputs,
        } = chunk_output;
        let (new_epoch, status, to_keep, to_discard, to_retry) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["sort_transactions"])
                .start_timer();
            // Separate transactions with different VM statuses.
            Self::sort_transactions(transactions, transaction_outputs)?
        };

        // Apply the write set, get the latest state.
        let (
            state_updates_vec,
            state_checkpoint_hashes,
            result_state,
            next_epoch_state,
            block_state_updates,
            sharded_state_cache,
        ) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["calculate_for_transaction_block"])
                .start_timer();
            InMemoryStateCalculatorV2::calculate_for_transaction_block(
                base_view.state(),
                state_cache,
                &to_keep,
                new_epoch,
            )?
        };

        // Calculate TransactionData and TransactionInfo, i.e. the ledger history diff.
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["assemble_ledger_diff_for_block"])
            .start_timer();
        let (to_commit, transaction_info_hashes, reconfig_events) =
            Self::assemble_ledger_diff_for_block(
                to_keep,
                state_updates_vec,
                state_checkpoint_hashes,
            );
        let result_view = ExecutedTrees::new(
            result_state,
            Arc::new(base_view.txn_accumulator().append(&transaction_info_hashes)),
        );

        Ok((
            ExecutedBlock {
                status,
                to_commit,
                result_view,
                next_epoch_state,
                reconfig_events,
                transaction_info_hashes,
                block_state_updates,
                sharded_state_cache,
            },
            to_discard,
            to_retry,
        ))
    }

    pub fn apply_chunk(
        chunk_output: ChunkOutput,
        base_view: &ExecutedTrees,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        let ChunkOutput {
            state_cache,
            transactions,
            transaction_outputs,
        } = chunk_output;
        let (new_epoch, status, to_keep, to_discard, to_retry) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["sort_transactions"])
                .start_timer();
            // Separate transactions with different VM statuses.
            Self::sort_transactions(transactions, transaction_outputs)?
        };

        // Apply the write set, get the latest state.
        let (state_updates_vec, state_checkpoint_hashes, result_state, next_epoch_state) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["calculate_for_transaction_chunk"])
                .start_timer();
            InMemoryStateCalculator::new(base_view.state(), state_cache)
                .calculate_for_transaction_chunk(&to_keep, new_epoch)?
        };

        // Calculate TransactionData and TransactionInfo, i.e. the ledger history diff.
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["assemble_ledger_diff_for_chunk"])
            .start_timer();
        let (to_commit, transaction_info_hashes) = Self::assemble_ledger_diff_for_chunk(
            to_keep,
            state_updates_vec,
            state_checkpoint_hashes,
        );
        let result_view = ExecutedTrees::new(
            result_state,
            Arc::new(base_view.txn_accumulator().append(&transaction_info_hashes)),
        );

        Ok((
            ExecutedChunk {
                status,
                to_commit,
                result_view,
                next_epoch_state,
                ledger_info: None,
            },
            to_discard,
            to_retry,
        ))
    }

    fn sort_transactions(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
    ) -> Result<(
        bool,
        Vec<TransactionStatus>,
        Vec<(Transaction, ParsedTransactionOutput)>,
        Vec<Transaction>,
        Vec<Transaction>,
    )> {
        let num_txns = transactions.len();
        let mut transaction_outputs: Vec<ParsedTransactionOutput> =
            transaction_outputs.into_iter().map(Into::into).collect();
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        // Transactions after the epoch ending are all to be retried.
        let to_retry = if let Some(pos) = new_epoch_marker {
            transaction_outputs.drain(pos..);
            transactions.drain(pos..).collect()
        } else {
            vec![]
        };

        // N.B. Transaction status after the epoch marker are ignored and set to Retry forcibly.
        let status = transaction_outputs
            .iter()
            .map(|t| t.status())
            .cloned()
            .chain(repeat(TransactionStatus::Retry))
            .take(num_txns)
            .collect();

        // Separate transactions with the Keep status out.
        let (to_keep, to_discard) =
            itertools::zip_eq(transactions.into_iter(), transaction_outputs.into_iter())
                .partition::<Vec<(Transaction, ParsedTransactionOutput)>, _>(|(_, o)| {
                    matches!(o.status(), TransactionStatus::Keep(_))
                });

        // Sanity check transactions with the Discard status:
        let to_discard = to_discard
            .into_iter()
            .map(|(t, o)| {
                // In case a new status other than Retry, Keep and Discard is added:
                if !matches!(o.status(), TransactionStatus::Discard(_)) {
                    error!("Status other than Retry, Keep or Discard; Transaction discarded.");
                }
                // VM shouldn't have output anything for discarded transactions, log if it did.
                if !o.write_set().is_empty() || !o.events().is_empty() {
                    error!(
                        "Discarded transaction has non-empty write set or events. \
                     Transaction: {:?}. Status: {:?}.",
                        t,
                        o.status(),
                    );
                    APTOS_EXECUTOR_ERRORS.inc();
                }
                Ok(t)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            new_epoch_marker.is_some(),
            status,
            to_keep,
            to_discard,
            to_retry,
        ))
    }

    fn assemble_ledger_diff_for_chunk(
        to_keep: Vec<(Transaction, ParsedTransactionOutput)>,
        state_updates_vec: Vec<HashMap<StateKey, Option<StateValue>>>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> (Vec<(Transaction, TransactionData)>, Vec<HashValue>) {
        // these are guaranteed by caller side logic
        assert_eq!(to_keep.len(), state_updates_vec.len());
        assert_eq!(to_keep.len(), state_checkpoint_hashes.len());

        let num_txns = to_keep.len();
        let mut to_commit = Vec::with_capacity(num_txns);
        let mut txn_info_hashes = Vec::with_capacity(num_txns);
        let hashes_vec = Self::calculate_events_and_writeset_hashes(&to_keep);

        for (
            (txn, txn_output),
            state_checkpoint_hash,
            state_updates,
            (event_hashes, write_set_hash),
        ) in itertools::izip!(
            to_keep,
            state_checkpoint_hashes,
            state_updates_vec,
            hashes_vec
        ) {
            let (write_set, events, reconfig_events, gas_used, status) = txn_output.unpack();
            let event_tree =
                InMemoryAccumulator::<EventAccumulatorHasher>::from_leaves(&event_hashes);

            let txn_info = match &status {
                TransactionStatus::Keep(status) => TransactionInfo::new(
                    txn.hash(),
                    write_set_hash,
                    event_tree.root_hash(),
                    state_checkpoint_hash,
                    gas_used,
                    status.clone(),
                ),
                _ => unreachable!("Transaction sorted by status already."),
            };
            let txn_info_hash = txn_info.hash();
            txn_info_hashes.push(txn_info_hash);
            to_commit.push((
                txn,
                TransactionData::new(
                    state_updates,
                    write_set,
                    events,
                    reconfig_events,
                    status,
                    Arc::new(event_tree),
                    gas_used,
                    txn_info,
                    txn_info_hash,
                ),
            ))
        }
        (to_commit, txn_info_hashes)
    }

    fn assemble_ledger_diff_for_block(
        to_keep: Vec<(Transaction, ParsedTransactionOutput)>,
        state_updates_vec: Vec<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> (
        Vec<Arc<TransactionToCommit>>,
        Vec<HashValue>,
        Vec<ContractEvent>,
    ) {
        // these are guaranteed by caller side logic
        assert_eq!(to_keep.len(), state_updates_vec.len());
        assert_eq!(to_keep.len(), state_checkpoint_hashes.len());

        let num_txns = to_keep.len();
        let mut to_commit = Vec::with_capacity(num_txns);
        let mut txn_info_hashes = Vec::with_capacity(num_txns);
        let hashes_vec = Self::calculate_events_and_writeset_hashes(&to_keep);
        let hashes_vec: Vec<(HashValue, HashValue)> = hashes_vec
            .into_par_iter()
            .map(|(event_hashes, write_set_hash)| {
                (
                    InMemoryAccumulator::<EventAccumulatorHasher>::from_leaves(&event_hashes)
                        .root_hash(),
                    write_set_hash,
                )
            })
            .collect();

        let mut all_reconfig_events = Vec::new();
        for (
            (txn, txn_output),
            state_checkpoint_hash,
            state_updates,
            (event_root_hash, write_set_hash),
        ) in itertools::izip!(
            to_keep,
            state_checkpoint_hashes,
            state_updates_vec,
            hashes_vec
        ) {
            let (write_set, events, per_txn_reconfig_events, gas_used, status) =
                txn_output.unpack();

            let txn_info = match &status {
                TransactionStatus::Keep(status) => TransactionInfo::new(
                    txn.hash(),
                    write_set_hash,
                    event_root_hash,
                    state_checkpoint_hash,
                    gas_used,
                    status.clone(),
                ),
                _ => unreachable!("Transaction sorted by status already."),
            };
            let txn_info_hash = txn_info.hash();
            txn_info_hashes.push(txn_info_hash);
            let txn_to_commit = TransactionToCommit::new(
                txn,
                txn_info,
                state_updates,
                write_set,
                events,
                !per_txn_reconfig_events.is_empty(),
            );
            all_reconfig_events.extend(per_txn_reconfig_events);
            to_commit.push(Arc::new(txn_to_commit));
        }
        (to_commit, txn_info_hashes, all_reconfig_events)
    }

    fn calculate_events_and_writeset_hashes(
        to_keep: &Vec<(Transaction, ParsedTransactionOutput)>,
    ) -> Vec<(Vec<HashValue>, HashValue)> {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_events_and_writeset_hashes"])
            .start_timer();
        to_keep
            .par_iter()
            .with_min_len(16)
            .map(|(_, txn_output)| {
                (
                    txn_output
                        .events()
                        .iter()
                        .map(CryptoHash::hash)
                        .collect::<Vec<_>>(),
                    CryptoHash::hash(txn_output.write_set()),
                )
            })
            .collect::<Vec<_>>()
    }
}

pub fn ensure_no_discard(to_discard: Vec<Transaction>) -> Result<()> {
    ensure!(to_discard.is_empty(), "Syncing discarded transactions");
    Ok(())
}

pub fn ensure_no_retry(to_retry: Vec<Transaction>) -> Result<()> {
    ensure!(to_retry.is_empty(), "Chunk crosses epoch boundary.",);
    Ok(())
}
