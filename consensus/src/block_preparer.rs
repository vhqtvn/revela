// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::{MAX_TXNS_FROM_BLOCK_TO_EXECUTE, TXN_SHUFFLE_SECONDS},
    payload_manager::PayloadManager,
    transaction_deduper::TransactionDeduper,
    transaction_filter::TransactionFilter,
    transaction_shuffler::TransactionShuffler,
};
use aptos_consensus_types::block::Block;
use aptos_executor_types::ExecutorResult;
use aptos_types::transaction::SignedTransaction;
use std::sync::Arc;

pub struct BlockPreparer {
    payload_manager: Arc<PayloadManager>,
    txn_filter: Arc<TransactionFilter>,
    txn_deduper: Arc<dyn TransactionDeduper>,
    txn_shuffler: Arc<dyn TransactionShuffler>,
}

impl BlockPreparer {
    pub fn new(
        payload_manager: Arc<PayloadManager>,
        txn_filter: Arc<TransactionFilter>,
        txn_deduper: Arc<dyn TransactionDeduper>,
        txn_shuffler: Arc<dyn TransactionShuffler>,
    ) -> Self {
        Self {
            payload_manager,
            txn_filter,
            txn_deduper,
            txn_shuffler,
        }
    }

    pub async fn prepare_block(&self, block: &Block) -> ExecutorResult<Vec<SignedTransaction>> {
        let (txns, max_txns_from_block_to_execute) =
            self.payload_manager.get_transactions(block).await?;
        let txn_filter = self.txn_filter.clone();
        let txn_deduper = self.txn_deduper.clone();
        let txn_shuffler = self.txn_shuffler.clone();
        let block_id = block.id();
        let block_timestamp_usecs = block.timestamp_usecs();
        // Transaction filtering, deduplication and shuffling are CPU intensive tasks, so we run them in a blocking task.
        tokio::task::spawn_blocking(move || {
            let filtered_txns = txn_filter.filter(block_id, block_timestamp_usecs, txns);
            let deduped_txns = txn_deduper.dedup(filtered_txns);
            let mut shuffled_txns = {
                let _timer = TXN_SHUFFLE_SECONDS.start_timer();

                txn_shuffler.shuffle(deduped_txns)
            };

            if let Some(max_txns_from_block_to_execute) = max_txns_from_block_to_execute {
                shuffled_txns.truncate(max_txns_from_block_to_execute);
            }
            MAX_TXNS_FROM_BLOCK_TO_EXECUTE.observe(shuffled_txns.len() as f64);
            Ok(shuffled_txns)
        })
        .await
        .expect("Failed to spawn blocking task for transaction generation")
    }
}
