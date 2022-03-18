// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bootstrapper::Bootstrapper,
    continuous_syncer::ContinuousSyncer,
    driver_client::{ClientNotificationListener, DriverNotification},
    error::Error,
    notification_handlers::{
        CommitNotification, CommitNotificationListener, CommittedAccounts, CommittedTransactions,
        ConsensusNotificationHandler, ErrorNotification, ErrorNotificationListener,
        MempoolNotificationHandler,
    },
    storage_synchronizer::StorageSynchronizerInterface,
    utils,
};
use aptos_config::config::{RoleType, StateSyncDriverConfig};
use aptos_data_client::AptosDataClient;
use aptos_infallible::Mutex;
use aptos_logger::*;
use aptos_types::waypoint::Waypoint;
use consensus_notifications::{
    ConsensusCommitNotification, ConsensusNotification, ConsensusSyncNotification,
};
use data_streaming_service::streaming_client::{NotificationFeedback, StreamingServiceClient};
use event_notifications::EventSubscriptionService;
use futures::StreamExt;
use mempool_notifications::MempoolNotificationSender;
use std::{sync::Arc, time::SystemTime};
use storage_interface::DbReader;
use tokio::time::{interval, Duration};
use tokio_stream::wrappers::IntervalStream;

// TODO(joshlind): use structured logging!

const MAX_CONNECTION_DEADLINE_SECS: u64 = 10;

/// The configuration of the state sync driver
#[derive(Clone)]
pub struct DriverConfiguration {
    // The config file of the driver
    pub config: StateSyncDriverConfig,

    // The role of the node
    pub role: RoleType,

    // The trusted waypoint for the node
    pub waypoint: Waypoint,
}

impl DriverConfiguration {
    pub fn new(config: StateSyncDriverConfig, role: RoleType, waypoint: Waypoint) -> Self {
        Self {
            config,
            role,
            waypoint,
        }
    }
}

/// The state sync driver that drives synchronization progress
pub struct StateSyncDriver<DataClient, MempoolNotifier, StorageSyncer> {
    // The component that manages the initial bootstrapping of the node
    bootstrapper: Bootstrapper<StorageSyncer>,

    // The listener for client notifications
    client_notification_listener: ClientNotificationListener,

    // The listener for commit notifications
    commit_notification_listener: CommitNotificationListener,

    // The handler for notifications from consensus
    consensus_notification_handler: ConsensusNotificationHandler,

    // The component that manages the continuous syncing of the node
    continuous_syncer: ContinuousSyncer<StorageSyncer>,

    // The client for checking the global data summary of our peers
    aptos_data_client: DataClient,

    // The configuration for the driver
    driver_configuration: DriverConfiguration,

    // The listener for errors from the storage synchronizer
    error_notification_listener: ErrorNotificationListener,

    // The event subscription service to notify listeners of on-chain events
    event_subscription_service: Arc<Mutex<EventSubscriptionService>>,

    // The handler for notifications to mempool
    mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,

    // The timestamp at which the driver started executing
    start_time: Option<SystemTime>,

    // The interface to read from storage
    storage: Arc<dyn DbReader>,
}

impl<
        DataClient: AptosDataClient + Send + Clone + 'static,
        MempoolNotifier: MempoolNotificationSender,
        StorageSyncer: StorageSynchronizerInterface + Clone,
    > StateSyncDriver<DataClient, MempoolNotifier, StorageSyncer>
{
    pub fn new(
        client_notification_listener: ClientNotificationListener,
        commit_notification_listener: CommitNotificationListener,
        consensus_notification_handler: ConsensusNotificationHandler,
        driver_configuration: DriverConfiguration,
        error_notification_listener: ErrorNotificationListener,
        event_subscription_service: EventSubscriptionService,
        mempool_notification_handler: MempoolNotificationHandler<MempoolNotifier>,
        storage_synchronizer: StorageSyncer,
        aptos_data_client: DataClient,
        streaming_service_client: StreamingServiceClient,
        storage: Arc<dyn DbReader>,
    ) -> Self {
        let event_subscription_service = Arc::new(Mutex::new(event_subscription_service));
        let bootstrapper = Bootstrapper::new(
            driver_configuration.clone(),
            streaming_service_client.clone(),
            storage.clone(),
            storage_synchronizer.clone(),
        );
        let continuous_syncer = ContinuousSyncer::new(
            driver_configuration.clone(),
            streaming_service_client,
            storage.clone(),
            storage_synchronizer,
        );

        Self {
            bootstrapper,
            client_notification_listener,
            commit_notification_listener,
            consensus_notification_handler,
            continuous_syncer,
            aptos_data_client,
            driver_configuration,
            error_notification_listener,
            event_subscription_service,
            mempool_notification_handler,
            start_time: None,
            storage,
        }
    }

    /// Starts the state sync driver
    pub async fn start_driver(mut self) {
        let mut progress_check_interval = IntervalStream::new(interval(Duration::from_millis(
            self.driver_configuration.config.progress_check_interval_ms,
        )))
        .fuse();

        // Start the driver
        info!("Started the state sync v2 driver!");
        self.start_time = Some(SystemTime::now());
        loop {
            ::futures::select! {
                notification = self.client_notification_listener.select_next_some() => {
                    self.handle_client_notification(notification);
                },
                notification = self.commit_notification_listener.select_next_some() => {
                    self.handle_commit_notification(notification).await;
                }
                notification = self.consensus_notification_handler.select_next_some() => {
                    self.handle_consensus_notification(notification).await;
                }
                notification = self.error_notification_listener.select_next_some() => {
                    self.handle_error_notification(notification).await;
                }
                _ = progress_check_interval.select_next_some() => {
                    self.drive_progress().await;
                }
            }
        }
    }

    /// Handles a notification sent by consensus
    async fn handle_consensus_notification(&mut self, notification: ConsensusNotification) {
        // Verify the notification: full nodes shouldn't receive notifications
        // and consensus should only send notifications after bootstrapping!
        let result = if self.driver_configuration.role == RoleType::FullNode {
            Err(Error::FullNodeConsensusNotification(format!(
                "Received consensus notification: {:?}",
                notification
            )))
        } else if !self.bootstrapper.is_bootstrapped() {
            Err(Error::BootstrapNotComplete(format!(
                "Received consensus notification: {:?}",
                notification
            )))
        } else {
            Ok(())
        };

        // Respond to consensus with any verification errors and then return
        if let Err(error) = result {
            match notification {
                ConsensusNotification::NotifyCommit(commit_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_commit_notification(commit_notification, Err(error.clone()))
                        .await;
                }
                ConsensusNotification::SyncToTarget(sync_notification) => {
                    let _ = self
                        .consensus_notification_handler
                        .respond_to_sync_notification(sync_notification, Err(error.clone()))
                        .await;
                }
            }
            error!(
                "Error encountered when handling the consensus notification: {:?}",
                error
            );
            return;
        }

        // Handle the notification
        let result = match notification {
            ConsensusNotification::NotifyCommit(commit_notification) => {
                self.handle_consensus_commit_notification(commit_notification)
                    .await
            }
            ConsensusNotification::SyncToTarget(sync_notification) => {
                self.handle_consensus_sync_notification(sync_notification)
                    .await
            }
        };

        // Log any errors from notification handling
        if let Err(error) = result {
            error!(
                "Error encountered when handling the consensus notification: {:?}",
                error
            );
        }
    }

    /// Handles a commit notification sent by consensus
    async fn handle_consensus_commit_notification(
        &mut self,
        consensus_commit_notification: ConsensusCommitNotification,
    ) -> Result<(), Error> {
        debug!(
            "Received a consensus commit notification! Transaction total: {:?}, event total: {:?}",
            consensus_commit_notification.transactions.len(),
            consensus_commit_notification.reconfiguration_events.len()
        );

        // TODO(joshlind): can we get consensus to forward the events?

        // Handle the commit notification
        let latest_synced_version = utils::fetch_latest_synced_version(self.storage.clone())?;
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        CommitNotification::handle_transaction_notification(
            consensus_commit_notification.reconfiguration_events.clone(),
            consensus_commit_notification.transactions.clone(),
            latest_synced_version,
            latest_synced_ledger_info,
            self.mempool_notification_handler.clone(),
            self.event_subscription_service.clone(),
        )
        .await?;

        // Respond to consensus successfully
        self.consensus_notification_handler
            .respond_to_commit_notification(consensus_commit_notification, Ok(()))
            .await?;

        // Check the progress of any sync requests. We need this here because
        // consensus might issue a sync request and then commit (asynchronously).
        self.check_sync_request_progress().await
    }

    /// Handles a consensus notification to sync to a specified target
    async fn handle_consensus_sync_notification(
        &mut self,
        sync_notification: ConsensusSyncNotification,
    ) -> Result<(), Error> {
        let latest_synced_version = utils::fetch_latest_synced_version(self.storage.clone())?;
        debug!(
            "Received a consensus sync notification! Target version: {:?}. Latest synced version: {:?}",
            sync_notification.target, latest_synced_version,
        );

        // Initialize a new sync request
        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        self.consensus_notification_handler
            .initialize_sync_request(sync_notification, latest_synced_ledger_info)
            .await
    }

    /// Handles a client notification sent by the driver client
    fn handle_client_notification(&mut self, notification: DriverNotification) {
        debug!("Received a notify bootstrap notification from the client!");

        // TODO(joshlind): refactor this if the client only supports one notification type!
        // Extract the bootstrap notifier channel
        let DriverNotification::NotifyOnceBootstrapped(notifier_channel) = notification;

        // Subscribe the bootstrap notifier channel
        if let Err(error) = self
            .bootstrapper
            .subscribe_to_bootstrap_notifications(notifier_channel)
        {
            error!(
                "Failed to subscribe to bootstrap notifications! Error: {:?}",
                error
            );
        }
    }

    /// Handles a commit notification sent by the storage synchronizer
    async fn handle_commit_notification(&mut self, commit_notification: CommitNotification) {
        match commit_notification {
            CommitNotification::CommittedAccounts(committed_accounts) => {
                debug!(
                    "Received an account commit notification from the storage synchronizer: {:?}",
                    committed_accounts
                );
                self.handle_committed_accounts(committed_accounts).await;
            }
            CommitNotification::CommittedTransactions(committed_transactions) => {
                debug!("Received a transaction commit notification from the storage synchronizer! Transaction total: {:?}, event total: {:?}",
                       committed_transactions.transactions.len(),
                       committed_transactions.events.len()
                );
                self.handle_committed_transactions(committed_transactions)
                    .await;
            }
        }
    }

    /// Handles a notification sent by the storage synchronizer for committed transactions
    async fn handle_committed_transactions(
        &mut self,
        committed_transactions: CommittedTransactions,
    ) {
        // Fetch the latest synced version and ledger info from storage
        let (latest_synced_version, latest_synced_ledger_info) =
            match utils::fetch_latest_synced_version(self.storage.clone()) {
                Ok(latest_synced_version) => {
                    match utils::fetch_latest_synced_ledger_info(self.storage.clone()) {
                        Ok(latest_synced_ledger_info) => {
                            (latest_synced_version, latest_synced_ledger_info)
                        }
                        Err(error) => {
                            error!(
                                "Failed to fetch latest synced ledger info! Error: {:?}",
                                error
                            );
                            return;
                        }
                    }
                }
                Err(error) => {
                    error!("Failed to fetch latest synced version! Error: {:?}", error);
                    return;
                }
            };

        // Handle the commit notification
        if let Err(error) = CommitNotification::handle_transaction_notification(
            committed_transactions.events,
            committed_transactions.transactions,
            latest_synced_version,
            latest_synced_ledger_info,
            self.mempool_notification_handler.clone(),
            self.event_subscription_service.clone(),
        )
        .await
        {
            error!(
                "Failed to handle a transaction commit notification! Error: {:?}",
                error
            );
        }

        // Update the last commit timestamp for the sync request
        let consensus_sync_request = self
            .consensus_notification_handler
            .get_consensus_sync_request();
        if let Some(sync_request) = consensus_sync_request.lock().as_mut() {
            sync_request.update_last_commit_timestamp()
        };
    }

    /// Handles a notification sent by the storage synchronizer for committed accounts
    async fn handle_committed_accounts(&mut self, committed_accounts: CommittedAccounts) {
        if let Err(error) = self
            .bootstrapper
            .handle_committed_accounts(committed_accounts)
        {
            error!(
                "Failed to handle an account commit notification! Error: {:?}",
                error
            );
        }
    }

    /// Handles an error notification sent by the storage synchronizer
    async fn handle_error_notification(&mut self, error_notification: ErrorNotification) {
        debug!(
            "Received an error notification from the storage synchronizer! Error: {:?}",
            error_notification.clone(),
        );

        // Terminate the currently active streams
        let notification_id = error_notification.notification_id;
        let notification_feedback = NotificationFeedback::InvalidPayloadData;
        if self.bootstrapper.is_bootstrapped() {
            if let Err(error) = self
                .continuous_syncer
                .terminate_active_stream(notification_id, notification_feedback)
                .await
            {
                panic!(
                    "Failed to terminate the active stream for the continuous syncer! Error: {:?}",
                    error
                );
            }
        } else if let Err(error) = self
            .bootstrapper
            .terminate_active_stream(notification_id, notification_feedback)
            .await
        {
            panic!(
                "Failed to terminate the active stream for the bootstrapper! Error: {:?}",
                error
            );
        };
    }

    /// Checks if the node has successfully reached the sync target
    async fn check_sync_request_progress(&mut self) -> Result<(), Error> {
        if !self.consensus_notification_handler.active_sync_request() {
            return Ok(());
        }

        let latest_synced_ledger_info =
            utils::fetch_latest_synced_ledger_info(self.storage.clone())?;
        self.consensus_notification_handler
            .check_sync_request_progress(latest_synced_ledger_info)
            .await
    }

    /// Returns true iff this node is a validator
    fn is_validator(&self) -> bool {
        self.driver_configuration.role == RoleType::Validator
    }

    /// Returns true iff consensus is currently executing
    fn check_if_consensus_executing(&self) -> bool {
        self.is_validator()
            && self.bootstrapper.is_bootstrapped()
            && !self.consensus_notification_handler.active_sync_request()
    }

    /// Checks if the connection deadline has passed. If so, validators with
    /// genesis waypoints will be automatically marked as bootstrapped. This
    /// helps in the case of single node deployments, where there are no peers
    /// and state sync is trivial.
    fn check_auto_bootstrapping(&mut self) {
        if !self.bootstrapper.is_bootstrapped()
            && self.is_validator()
            && self.driver_configuration.waypoint.version() == 0
        {
            if let Some(start_time) = self.start_time {
                if let Some(connection_deadline) =
                    start_time.checked_add(Duration::from_secs(MAX_CONNECTION_DEADLINE_SECS))
                {
                    if SystemTime::now()
                        .duration_since(connection_deadline)
                        .is_ok()
                    {
                        info!("Passed the connection deadline! Auto-bootstrapping the validator!");
                        if let Err(error) = self.bootstrapper.bootstrapping_complete() {
                            error!(
                                "Failed to mark bootstrapping as complete! Error: {:?}",
                                error
                            );
                        }
                    }
                } else {
                    error!("The connection deadline overflowed! Unable to auto-bootstrap!");
                }
            }
        }
    }

    /// Checks that state sync is making progress
    async fn drive_progress(&mut self) {
        // Fetch the global data summary and verify we have active peers
        let global_data_summary = self.aptos_data_client.get_global_data_summary();
        if global_data_summary.is_empty() {
            trace!("The global data summary is empty! It's likely that we have no active peers.");
            return self.check_auto_bootstrapping();
        }

        // Check the progress of any sync requests
        if let Err(error) = self.check_sync_request_progress().await {
            error!(
                "Error found when checking the sync request progress: {:?}",
                error
            );
        }

        // If consensus is executing, there's nothing to do
        if self.check_if_consensus_executing() {
            trace!("Consensus is executing. There's nothing to do.");
            return;
        }

        // Drive progress depending on if we're bootstrapping or continuously syncing
        if self.bootstrapper.is_bootstrapped() {
            let consensus_sync_request = self
                .consensus_notification_handler
                .get_consensus_sync_request();
            if let Err(error) = self
                .continuous_syncer
                .drive_progress(consensus_sync_request)
                .await
            {
                error!(
                    "Error found when driving progress of the continuous syncer: {:?}",
                    error
                );
            }
        } else if let Err(error) = self.bootstrapper.drive_progress(&global_data_summary).await {
            error!(
                "Error found when checking the bootstrapper progress: {:?}",
                error
            );
        };
    }
}
