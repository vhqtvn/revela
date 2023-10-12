// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{
        chunk_output::{update_counters_for_processed_chunk, ChunkOutput},
        in_memory_state_calculator_v2::InMemoryStateCalculatorV2,
    },
    metrics::{APTOS_EXECUTOR_ERRORS, APTOS_EXECUTOR_OTHER_TIMERS_SECONDS},
};
use anyhow::{ensure, Result};
use aptos_crypto::{
    hash::{CryptoHash, EventAccumulatorHasher, TransactionAccumulatorHasher},
    HashValue,
};
use aptos_executor_types::{
    parsed_transaction_output::TransactionsWithParsedOutput,
    state_checkpoint_output::{StateCheckpointOutput, TransactionsByStatus},
    ExecutedChunk, LedgerUpdateOutput, ParsedTransactionOutput,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::error;
use aptos_storage_interface::{state_delta::StateDelta, ExecutedTrees};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::accumulator::InMemoryAccumulator,
    state_store::ShardedStateUpdates,
    transaction::{
        ExecutionStatus, Transaction, TransactionInfo, TransactionOutput, TransactionStatus,
        TransactionToCommit,
    },
    write_set::WriteSet,
};
use rayon::prelude::*;
use std::{
    iter::{once, repeat},
    sync::Arc,
};

pub struct ApplyChunkOutput;

impl ApplyChunkOutput {
    pub fn calculate_state_checkpoint(
        chunk_output: ChunkOutput,
        parent_state: &StateDelta,
        append_state_checkpoint_to_block: Option<HashValue>,
        is_block: bool,
    ) -> Result<(StateDelta, Option<EpochState>, StateCheckpointOutput)> {
        let ChunkOutput {
            state_cache,
            transactions,
            transaction_outputs,
        } = chunk_output;
        let (new_epoch, status, to_keep, to_discard, to_retry) = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["sort_transactions"])
                .start_timer();
            // Separate transactions with different VM statuses, i.e., Keep, Discard and Retry.
            // Will return transactions with Retry txns sorted after Keep/Discard txns.
            // If the transactions contain no reconfiguration txn, will insert the StateCheckpoint txn
            // at the boundary of Keep/Discard txns and Retry txns.
            Self::sort_transactions_with_state_checkpoint(
                transactions,
                transaction_outputs,
                append_state_checkpoint_to_block,
            )?
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
                .with_label_values(&["calculate_for_transactions"])
                .start_timer();
            InMemoryStateCalculatorV2::calculate_for_transactions(
                parent_state,
                state_cache,
                &to_keep,
                new_epoch,
                is_block,
            )?
        };

        Ok((
            result_state,
            next_epoch_state,
            StateCheckpointOutput::new(
                TransactionsByStatus::new(status, to_keep, to_discard, to_retry),
                state_updates_vec,
                state_checkpoint_hashes,
                block_state_updates,
                sharded_state_cache,
            ),
        ))
    }

    pub fn calculate_ledger_update(
        state_checkpoint_output: StateCheckpointOutput,
        base_txn_accumulator: Arc<InMemoryAccumulator<TransactionAccumulatorHasher>>,
    ) -> Result<(LedgerUpdateOutput, Vec<Transaction>, Vec<Transaction>)> {
        let (
            txns,
            state_updates_vec,
            state_checkpoint_hashes,
            state_updates_before_last_checkpoint,
            sharded_state_cache,
        ) = state_checkpoint_output.into_inner();

        let (status, to_keep, to_discard, to_retry) = txns.into_inner();

        update_counters_for_processed_chunk(to_keep.txns(), to_keep.parsed_outputs(), "execution");
        update_counters_for_processed_chunk(
            to_discard.txns(),
            to_discard.parsed_outputs(),
            "execution",
        );
        update_counters_for_processed_chunk(
            to_retry.txns(),
            to_retry.parsed_outputs(),
            "execution",
        );

        // Calculate TransactionData and TransactionInfo, i.e. the ledger history diff.
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["assemble_ledger_diff_for_block"])
            .start_timer();

        let (to_commit, transaction_info_hashes, reconfig_events) =
            Self::assemble_ledger_diff(to_keep, state_updates_vec, state_checkpoint_hashes);
        let transaction_accumulator =
            Arc::new(base_txn_accumulator.append(&transaction_info_hashes));
        Ok((
            LedgerUpdateOutput {
                status,
                to_commit,
                reconfig_events,
                transaction_info_hashes,
                state_updates_before_last_checkpoint,
                sharded_state_cache,
                transaction_accumulator,
            },
            to_discard.into_txns(),
            to_retry.into_txns(),
        ))
    }

    pub fn apply_chunk(
        chunk_output: ChunkOutput,
        base_view: &ExecutedTrees,
        state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<(ExecutedChunk, Vec<Transaction>, Vec<Transaction>)> {
        let (result_state, next_epoch_state, mut state_checkpoint_output) =
            Self::calculate_state_checkpoint(
                chunk_output,
                base_view.state(),
                append_state_checkpoint_to_block,
                /*is_block=*/ false,
            )?;

        // On state sync/replay, we generate state checkpoints only periodically, for the
        // last state checkpoint of each chunk.
        // A mismatch in the SMT will be detected at that occasion too. Here we just copy
        // in the state root from the TxnInfo in the proof.
        if let Some(state_checkpoint_hashes) = state_checkpoint_hashes {
            state_checkpoint_output
                .check_and_update_state_checkpoint_hashes(state_checkpoint_hashes)?;
        }

        let (ledger_update_output, to_discard, to_retry) = Self::calculate_ledger_update(
            state_checkpoint_output,
            base_view.txn_accumulator().clone(),
        )?;

        Ok((
            ExecutedChunk {
                status: ledger_update_output.status,
                to_commit: ledger_update_output.to_commit,
                result_view: ExecutedTrees::new(
                    result_state,
                    ledger_update_output.transaction_accumulator,
                ),
                next_epoch_state,
                ledger_info: None,
            },
            to_discard,
            to_retry,
        ))
    }

    fn sort_transactions_with_state_checkpoint(
        mut transactions: Vec<Transaction>,
        transaction_outputs: Vec<TransactionOutput>,
        append_state_checkpoint_to_block: Option<HashValue>,
    ) -> Result<(
        bool,
        Vec<TransactionStatus>,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
        TransactionsWithParsedOutput,
    )> {
        let mut transaction_outputs: Vec<ParsedTransactionOutput> =
            transaction_outputs.into_iter().map(Into::into).collect();
        // N.B. off-by-1 intentionally, for exclusive index
        let new_epoch_marker = transaction_outputs
            .iter()
            .position(|o| o.is_reconfig())
            .map(|idx| idx + 1);

        let block_gas_limit_marker = transaction_outputs
            .iter()
            .position(|o| matches!(o.status(), TransactionStatus::Retry));

        // Transactions after the epoch ending txn are all to be retried.
        // Transactions after the txn that exceeded per-block gas limit are also to be retried.
        let to_retry = if let Some(pos) = new_epoch_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else if let Some(pos) = block_gas_limit_marker {
            TransactionsWithParsedOutput::new(
                transactions.drain(pos..).collect(),
                transaction_outputs.drain(pos..).collect(),
            )
        } else {
            TransactionsWithParsedOutput::new(vec![], vec![])
        };

        let state_checkpoint_to_add =
            new_epoch_marker.map_or_else(|| append_state_checkpoint_to_block, |_| None);

        let keeps_and_discards = transaction_outputs.iter().map(|t| t.status()).cloned();
        let retries = repeat(TransactionStatus::Retry).take(to_retry.len());

        let status = if state_checkpoint_to_add.is_some() {
            keeps_and_discards
                .chain(once(TransactionStatus::Keep(ExecutionStatus::Success)))
                .chain(retries)
                .collect()
        } else {
            keeps_and_discards.chain(retries).collect()
        };

        // Separate transactions with the Keep status out.
        let (mut to_keep, to_discard) = itertools::zip_eq(transactions, transaction_outputs)
            .partition::<Vec<(Transaction, ParsedTransactionOutput)>, _>(|(_, o)| {
                matches!(o.status(), TransactionStatus::Keep(_))
            });

        // Append the StateCheckpoint transaction to the end of to_keep
        if let Some(block_id) = state_checkpoint_to_add {
            let state_checkpoint_txn = Transaction::StateCheckpoint(block_id);
            let state_checkpoint_txn_output: ParsedTransactionOutput =
                Into::into(TransactionOutput::new(
                    WriteSet::default(),
                    Vec::new(),
                    0,
                    TransactionStatus::Keep(ExecutionStatus::Success),
                ));
            to_keep.push((state_checkpoint_txn, state_checkpoint_txn_output));
        }

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
                Ok((t, o))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            new_epoch_marker.is_some(),
            status,
            to_keep.into(),
            to_discard.into(),
            to_retry,
        ))
    }

    fn assemble_ledger_diff(
        to_keep: TransactionsWithParsedOutput,
        state_updates_vec: Vec<ShardedStateUpdates>,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> (Vec<TransactionToCommit>, Vec<HashValue>, Vec<ContractEvent>) {
        // these are guaranteed by caller side logic
        assert_eq!(to_keep.len(), state_updates_vec.len());
        assert_eq!(to_keep.len(), state_checkpoint_hashes.len());

        let num_txns = to_keep.len();
        let mut to_commit = Vec::with_capacity(num_txns);
        let mut txn_info_hashes = Vec::with_capacity(num_txns);
        let hashes_vec = Self::calculate_events_and_writeset_hashes(to_keep.parsed_outputs());
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
        let (to_keep_txns, to_keep_outputs) = to_keep.into_inner();
        for (
            txn,
            txn_output,
            state_checkpoint_hash,
            state_updates,
            (event_root_hash, write_set_hash),
        ) in itertools::izip!(
            to_keep_txns,
            to_keep_outputs,
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
            to_commit.push(txn_to_commit);
        }
        (to_commit, txn_info_hashes, all_reconfig_events)
    }

    fn calculate_events_and_writeset_hashes(
        to_keep: &[ParsedTransactionOutput],
    ) -> Vec<(Vec<HashValue>, HashValue)> {
        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["calculate_events_and_writeset_hashes"])
            .start_timer();
        let num_txns = to_keep.len();
        to_keep
            .par_iter()
            .with_min_len(optimal_min_len(num_txns, 64))
            .map(|txn_output| {
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
