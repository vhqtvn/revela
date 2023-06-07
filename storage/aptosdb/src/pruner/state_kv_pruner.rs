// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_metadata::DbMetadataSchema,
    metrics::PRUNER_VERSIONS,
    pruner::{
        db_pruner::DBPruner, db_sub_pruner::DBSubPruner,
        state_store::state_value_pruner::StateValuePruner,
    },
    pruner_utils,
    schema::db_metadata::{DbMetadataKey, DbMetadataValue},
    state_kv_db::StateKvDb,
};
use anyhow::Result;
use aptos_schemadb::SchemaBatch;
use aptos_types::transaction::{AtomicVersion, Version};
use std::sync::{atomic::Ordering, Arc};

pub const STATE_KV_PRUNER_NAME: &str = "state_kv_pruner";

/// Responsible for pruning state kv db.
pub(crate) struct StateKvPruner {
    state_kv_db: Arc<StateKvDb>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    min_readable_version: AtomicVersion,
    state_value_pruner: Arc<dyn DBSubPruner + Send + Sync>,
}

impl DBPruner for StateKvPruner {
    fn name(&self) -> &'static str {
        STATE_KV_PRUNER_NAME
    }

    fn prune(&self, max_versions: usize) -> Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.min_readable_version());
        }

        let mut db_batch = SchemaBatch::new();
        let current_target_version = self.prune_inner(max_versions, &mut db_batch)?;
        self.save_min_readable_version(current_target_version, &db_batch)?;
        self.state_kv_db.commit_raw_batch(db_batch)?;
        self.record_progress(current_target_version);

        Ok(current_target_version)
    }

    fn save_min_readable_version(&self, version: Version, batch: &SchemaBatch) -> Result<()> {
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    fn initialize_min_readable_version(&self) -> anyhow::Result<Version> {
        Ok(self
            .state_kv_db
            .metadata_db()
            .get::<DbMetadataSchema>(&DbMetadataKey::StateKvPrunerProgress)?
            .map_or(0, |v| v.expect_version()))
    }

    fn min_readable_version(&self) -> Version {
        self.min_readable_version.load(Ordering::Relaxed)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed);
        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "target"])
            .set(target_version as i64);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    fn record_progress(&self, min_readable_version: Version) {
        self.min_readable_version
            .store(min_readable_version, Ordering::Relaxed);
        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "progress"])
            .set(min_readable_version as i64);
    }
}

impl StateKvPruner {
    pub fn new(state_kv_db: Arc<StateKvDb>) -> Self {
        let pruner = StateKvPruner {
            state_kv_db: Arc::clone(&state_kv_db),
            target_version: AtomicVersion::new(0),
            min_readable_version: AtomicVersion::new(0),
            state_value_pruner: Arc::new(StateValuePruner::new(state_kv_db)),
        };
        pruner.initialize();
        pruner
    }

    /// Prunes the genesis transaction and saves the db alterations to the given change set
    pub fn prune_genesis(
        state_kv_db: Arc<StateKvDb>,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<()> {
        let target_version = 1; // The genesis version is 0. Delete [0,1) (exclusive)
        let max_version = 1; // We should only be pruning a single version

        let state_kv_pruner = pruner_utils::create_state_kv_pruner(state_kv_db);
        state_kv_pruner.set_target_version(target_version);
        state_kv_pruner.prune_inner(max_version, db_batch)?;

        Ok(())
    }

    fn prune_inner(
        &self,
        max_versions: usize,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<Version> {
        let min_readable_version = self.min_readable_version();

        let current_target_version = self.get_current_batch_target(max_versions as Version);
        if current_target_version < min_readable_version {
            return Ok(min_readable_version);
        }

        self.state_value_pruner
            .prune(db_batch, min_readable_version, current_target_version)?;

        Ok(current_target_version)
    }
}
