// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_backup_service::start_backup_service;
use aptos_config::{config::NodeConfig, utils::get_genesis_txn};
use aptos_db::{fast_sync_storage_wrapper::FastSyncStorageWrapper, AptosDB};
use aptos_executor::db_bootstrapper::maybe_bootstrap;
use aptos_logger::{debug, info};
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::waypoint::Waypoint;
use aptos_vm::AptosVM;
use either::Either;
use std::{fs, path::Path, sync::Arc, time::Instant};
use tokio::runtime::Runtime;

pub(crate) fn maybe_apply_genesis(db_rw: &DbReaderWriter, node_config: &NodeConfig) -> Result<()> {
    let genesis_waypoint = node_config.base.waypoint.genesis_waypoint();
    if let Some(genesis) = get_genesis_txn(node_config) {
        maybe_bootstrap::<AptosVM>(db_rw, genesis, genesis_waypoint)
            .map_err(|err| anyhow!("DB failed to bootstrap {}", err))?;
    } else {
        info ! ("Genesis txn not provided! This is fine only if you don't expect to apply it. Otherwise, the config is incorrect!");
    }
    Ok(())
}

#[cfg(not(feature = "consensus-only-perf-test"))]
pub(crate) fn bootstrap_db(
    node_config: &NodeConfig,
) -> Result<(Arc<dyn DbReader>, DbReaderWriter, Option<Runtime>)> {
    let (aptos_db_reader, db_rw, backup_service) =
        match FastSyncStorageWrapper::initialize_dbs(node_config)? {
            Either::Left(db) => {
                let (db_arc, db_rw) = DbReaderWriter::wrap(db);
                let db_backup_service = start_backup_service(
                    node_config.storage.backup_service_address,
                    db_arc.clone(),
                );
                maybe_apply_genesis(&db_rw, node_config)?;
                (db_arc as Arc<dyn DbReader>, db_rw, Some(db_backup_service))
            },
            Either::Right(fast_sync_db_wrapper) => {
                let temp_db = fast_sync_db_wrapper.get_temporary_db_with_genesis();
                maybe_apply_genesis(&DbReaderWriter::from_arc(temp_db), node_config)?;

                let (db_arc, db_rw) = DbReaderWriter::wrap(fast_sync_db_wrapper);
                let fast_sync_db = db_arc.get_fast_sync_db();
                let db_backup_service =
                    start_backup_service(node_config.storage.backup_service_address, fast_sync_db);

                (db_arc as Arc<dyn DbReader>, db_rw, Some(db_backup_service))
            },
        };

    Ok((aptos_db_reader, db_rw, backup_service))
}

/// In consensus-only mode, return a in-memory based [FakeAptosDB] and
/// do not run the backup service.
#[cfg(feature = "consensus-only-perf-test")]
pub(crate) fn bootstrap_db(
    node_config: &NodeConfig,
) -> Result<(Arc<dyn DbReader>, DbReaderWriter, Option<Runtime>)> {
    use aptos_db::fake_aptosdb::FakeAptosDB;

    let aptos_db = AptosDB::open(
        &node_config.storage.dir(),
        false, /* readonly */
        node_config.storage.storage_pruner_config,
        node_config.storage.rocksdb_configs,
        node_config.storage.enable_indexer,
        node_config.storage.buffered_state_target_items,
        node_config.storage.max_num_nodes_per_lru_cache_shard,
    )
    .map_err(|err| anyhow!("DB failed to open {}", err))?;
    let (aptos_db, db_rw) = DbReaderWriter::wrap(FakeAptosDB::new(aptos_db));
    maybe_apply_genesis(&db_rw, node_config)?;
    Ok((aptos_db, db_rw, None))
}

/// Creates a RocksDb checkpoint for the consensus_db, state_sync_db,
/// ledger_db and state_merkle_db and saves it to the checkpoint_path.
/// Also, changes the working directory to run the node on the new path,
/// so that the existing data won't change. For now this is a test-only feature.
fn create_rocksdb_checkpoint_and_change_working_dir(
    node_config: &mut NodeConfig,
    working_dir: impl AsRef<Path>,
) {
    // Update the source and checkpoint directories
    let source_dir = node_config.storage.dir();
    node_config.set_data_dir(working_dir.as_ref().to_path_buf());
    let checkpoint_dir = node_config.storage.dir();
    assert!(source_dir != checkpoint_dir);

    // Create rocksdb checkpoint directory
    fs::create_dir_all(&checkpoint_dir).unwrap();

    // Open the database and create a checkpoint
    AptosDB::create_checkpoint(
        &source_dir,
        &checkpoint_dir,
        node_config.storage.rocksdb_configs.enable_storage_sharding,
    )
    .expect("AptosDB checkpoint creation failed.");

    // Create a consensus db checkpoint
    aptos_consensus::create_checkpoint(&source_dir, &checkpoint_dir)
        .expect("ConsensusDB checkpoint creation failed.");

    // Create a state sync db checkpoint
    let state_sync_db =
        aptos_state_sync_driver::metadata_storage::PersistentMetadataStorage::new(&source_dir);
    state_sync_db
        .create_checkpoint(&checkpoint_dir)
        .expect("StateSyncDB checkpoint creation failed.");
}

/// Creates any rocksdb checkpoints, opens the storage database,
/// starts the backup service, handles genesis initialization and returns
/// the various handles.
pub fn initialize_database_and_checkpoints(
    node_config: &mut NodeConfig,
) -> Result<(Arc<dyn DbReader>, DbReaderWriter, Option<Runtime>, Waypoint)> {
    // If required, create RocksDB checkpoints and change the working directory.
    // This is test-only.
    if let Some(working_dir) = node_config.base.working_dir.clone() {
        create_rocksdb_checkpoint_and_change_working_dir(node_config, working_dir);
    }

    // Open the database
    let instant = Instant::now();
    let (aptos_db, db_rw, backup_service) = bootstrap_db(node_config)?;

    // Log the duration to open storage
    debug!(
        "Storage service started in {} ms",
        instant.elapsed().as_millis()
    );

    Ok((
        aptos_db,
        db_rw,
        backup_service,
        node_config.base.waypoint.genesis_waypoint(),
    ))
}
