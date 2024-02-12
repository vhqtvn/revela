// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::table_info_service::TableInfoService;
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_db_indexer::{db_ops::open_db, db_v2::IndexerAsyncV2};
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::chain_id::ChainId;
use std::sync::Arc;
use tokio::runtime::Runtime;

const INDEX_ASYNC_V2_DB_NAME: &str = "index_indexer_async_v2_db";

/// Creates a runtime which creates a thread pool which sets up fullnode indexer table info service
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db_rw: DbReaderWriter,
    mp_sender: MempoolClientSender,
) -> Option<(Runtime, Arc<IndexerAsyncV2>)> {
    if !config.indexer_table_info.enabled {
        return None;
    }

    let runtime = aptos_runtimes::spawn_named_runtime("table-info".to_string(), None);

    // Set up db config and open up the db initially to read metadata
    let node_config = config.clone();
    let db_path = node_config
        .storage
        .get_dir_paths()
        .default_root_path()
        .join(INDEX_ASYNC_V2_DB_NAME);
    let rocksdb_config = node_config.storage.rocksdb_configs.index_db_config;
    let db =
        open_db(db_path, &rocksdb_config).expect("Failed to open up indexer async v2 db initially");

    let indexer_async_v2 =
        Arc::new(IndexerAsyncV2::new(db).expect("Failed to initialize indexer async v2"));
    let indexer_async_v2_clone = Arc::clone(&indexer_async_v2);

    // Spawn the runtime for table info parsing
    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db_rw.reader.clone(),
            mp_sender,
            node_config.clone(),
            None,
        ));

        let mut parser = TableInfoService::new(
            context,
            indexer_async_v2_clone.next_version(),
            node_config.indexer_table_info.parser_task_count,
            node_config.indexer_table_info.parser_batch_size,
            node_config.indexer_table_info.enable_expensive_logging,
            indexer_async_v2_clone,
        );

        parser.run().await;
    });

    Some((runtime, indexer_async_v2))
}
