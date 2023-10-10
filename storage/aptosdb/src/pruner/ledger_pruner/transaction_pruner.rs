// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction::TransactionSchema,
    },
    TransactionStore,
};
use anyhow::{ensure, Result};
use aptos_logger::info;
use aptos_schemadb::{ReadOptions, SchemaBatch, DB};
use aptos_types::transaction::{Transaction, Version};
use std::sync::Arc;

#[derive(Debug)]
pub struct TransactionPruner {
    transaction_store: Arc<TransactionStore>,
    transaction_db: Arc<DB>,
}

impl DBSubPruner for TransactionPruner {
    fn name(&self) -> &str {
        "TransactionPruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        let candidate_transactions =
            self.get_pruning_candidate_transactions(current_progress, target_version)?;
        self.transaction_store
            .prune_transaction_by_hash(&candidate_transactions, &batch)?;
        self.transaction_store
            .prune_transaction_by_account(&candidate_transactions, &batch)?;
        self.transaction_store.prune_transaction_schema(
            current_progress,
            target_version,
            &batch,
        )?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.transaction_db.write_schemas(batch)
    }
}

impl TransactionPruner {
    pub(in crate::pruner) fn new(
        transaction_store: Arc<TransactionStore>,
        transaction_db: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &transaction_db,
            &DbMetadataKey::TransactionPrunerProgress,
            metadata_progress,
        )?;

        let myself = TransactionPruner {
            transaction_store,
            transaction_db,
        };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up TransactionPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }

    fn get_pruning_candidate_transactions(
        &self,
        start: Version,
        end: Version,
    ) -> Result<Vec<Transaction>> {
        ensure!(end >= start);

        let mut iter = self
            .transaction_db
            .iter::<TransactionSchema>(ReadOptions::default())?;
        iter.seek(&start)?;

        // The capacity is capped by the max number of txns we prune in a single batch. It's a
        // relatively small number set in the config, so it won't cause high memory usage here.
        let mut txns = Vec::with_capacity((end - start) as usize);
        for item in iter {
            let (version, txn) = item?;
            if version >= end {
                break;
            }
            txns.push(txn);
        }

        Ok(txns)
    }
}
