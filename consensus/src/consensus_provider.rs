// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{network::ObserverMessage, observer::Observer, publisher::Publisher},
    counters,
    epoch_manager::EpochManager,
    network::NetworkTask,
    network_interface::{ConsensusMsg, ConsensusNetworkClient},
    persistent_liveness_storage::StorageWriteProxy,
    pipeline::execution_client::ExecutionProxyClient,
    quorum_store::quorum_store_db::QuorumStoreDB,
    rand::rand_gen::storage::db::RandDb,
    state_computer::ExecutionProxy,
    transaction_filter::TransactionFilter,
    txn_notifier::MempoolNotifier,
    util::time_service::ClockTimeService,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::config::NodeConfig;
use aptos_consensus_notifications::ConsensusNotificationSender;
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_executor::block_executor::BlockExecutor;
use aptos_logger::prelude::*;
use aptos_mempool::QuorumStoreRequest;
use aptos_network::application::interface::{
    NetworkClient, NetworkClientInterface, NetworkServiceEvents,
};
use aptos_storage_interface::DbReaderWriter;
use aptos_validator_transaction_pool::VTxnPoolState;
use aptos_vm::AptosVM;
use futures::{channel::mpsc, stream::select_all};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, sync::Arc};
use tokio::runtime::Runtime;

/// Helper function to start consensus based on configuration and return the runtime
pub fn start_consensus(
    node_config: &NodeConfig,
    network_client: NetworkClient<ConsensusMsg>,
    network_service_events: NetworkServiceEvents<ConsensusMsg>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
    aptos_db: DbReaderWriter,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    vtxn_pool: VTxnPoolState,
    observer_network: Option<NetworkClient<ObserverMessage>>,
) -> (Runtime, Arc<StorageWriteProxy>, Arc<QuorumStoreDB>) {
    let runtime = aptos_runtimes::spawn_named_runtime("consensus".into(), None);
    let storage = Arc::new(StorageWriteProxy::new(node_config, aptos_db.reader.clone()));
    let quorum_store_db = Arc::new(QuorumStoreDB::new(node_config.storage.dir()));

    let txn_notifier = Arc::new(MempoolNotifier::new(
        consensus_to_mempool_sender.clone(),
        node_config.consensus.mempool_executed_txn_timeout_ms,
    ));

    let execution_proxy = ExecutionProxy::new(
        Arc::new(BlockExecutor::<AptosVM>::new(aptos_db)),
        txn_notifier,
        state_sync_notifier,
        runtime.handle(),
        TransactionFilter::new(node_config.execution.transaction_filter.clone()),
    );

    let time_service = Arc::new(ClockTimeService::new(runtime.handle().clone()));

    let (timeout_sender, timeout_receiver) =
        aptos_channels::new(1_024, &counters::PENDING_ROUND_TIMEOUTS);
    let (self_sender, self_receiver) =
        aptos_channels::new_unbounded(&counters::PENDING_SELF_MESSAGES);
    let consensus_network_client = ConsensusNetworkClient::new(network_client);
    let bounded_executor = BoundedExecutor::new(8, runtime.handle().clone());
    let rand_storage = Arc::new(RandDb::new(node_config.storage.dir()));

    let execution_client = Arc::new(ExecutionProxyClient::new(
        node_config.consensus.clone(),
        Arc::new(execution_proxy),
        node_config.validator_network.as_ref().unwrap().peer_id(),
        self_sender.clone(),
        consensus_network_client.clone(),
        bounded_executor.clone(),
        rand_storage.clone(),
        observer_network.clone(),
    ));

    let epoch_mgr = EpochManager::new(
        node_config,
        time_service,
        self_sender,
        consensus_network_client,
        timeout_sender,
        consensus_to_mempool_sender,
        execution_client,
        storage.clone(),
        quorum_store_db.clone(),
        reconfig_events,
        bounded_executor,
        aptos_time_service::TimeService::real(),
        vtxn_pool,
        rand_storage,
        observer_network,
    );

    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);

    runtime.spawn(network_task.start());
    runtime.spawn(epoch_mgr.start(timeout_receiver, network_receiver));

    debug!("Consensus started.");
    (runtime, storage, quorum_store_db)
}

pub fn start_consensus_observer(
    node_config: &NodeConfig,
    observer_network_client: NetworkClient<ObserverMessage>,
    observer_network_service_events: NetworkServiceEvents<ObserverMessage>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    consensus_to_mempool_sender: mpsc::Sender<QuorumStoreRequest>,
    aptos_db: DbReaderWriter,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
) -> Runtime {
    let publisher_enabled = node_config.consensus_observer.publisher_enabled;
    let runtime = aptos_runtimes::spawn_named_runtime("observer".into(), None);
    let root = aptos_db.reader.get_latest_ledger_info().unwrap();

    let txn_notifier = Arc::new(MempoolNotifier::new(
        consensus_to_mempool_sender.clone(),
        node_config.consensus.mempool_executed_txn_timeout_ms,
    ));

    let execution_proxy = ExecutionProxy::new(
        Arc::new(BlockExecutor::<AptosVM>::new(aptos_db)),
        txn_notifier,
        state_sync_notifier,
        runtime.handle(),
        TransactionFilter::new(node_config.execution.transaction_filter.clone()),
    );

    let (self_sender, _self_receiver) =
        aptos_channels::new_unbounded(&counters::PENDING_SELF_MESSAGES);
    let dummy_client = ConsensusNetworkClient::new(NetworkClient::new(
        vec![],
        vec![],
        HashMap::new(),
        observer_network_client.get_peers_and_metadata(),
    ));
    let bounded_executor = BoundedExecutor::new(32, runtime.handle().clone());
    let rand_storage = Arc::new(RandDb::new(node_config.storage.dir()));

    let execution_client = Arc::new(ExecutionProxyClient::new(
        node_config.consensus.clone(),
        Arc::new(execution_proxy),
        AccountAddress::ONE,
        self_sender.clone(),
        dummy_client,
        bounded_executor.clone(),
        rand_storage.clone(),
        if publisher_enabled {
            Some(observer_network_client.clone())
        } else {
            None
        },
    ));

    let events: Vec<_> = observer_network_service_events
        .into_network_and_events()
        .into_values()
        .collect();
    let network_events = Box::new(select_all(events));

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let observer = Observer::new(
        root,
        execution_client,
        tx,
        reconfig_events,
        if publisher_enabled {
            Some(Publisher::new(observer_network_client))
        } else {
            None
        },
    );
    runtime.spawn(observer.start(network_events, rx));
    runtime
}
