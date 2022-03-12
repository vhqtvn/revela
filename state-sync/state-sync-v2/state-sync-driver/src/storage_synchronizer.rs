// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    notification_handlers::{CommitNotification, ErrorNotification},
};
use aptos_logger::prelude::*;
use aptos_types::{
    account_state_blob::AccountStatesChunkWithProof,
    ledger_info::LedgerInfoWithSignatures,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof},
};
use data_streaming_service::data_notification::NotificationId;
use executor_types::ChunkExecutorTrait;
use futures::{channel::mpsc, SinkExt, StreamExt};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::runtime::Runtime;

// TODO(joshlind): add structured logging support!

// The maximum number of chunks that are pending execution or commit
const MAX_PENDING_CHUNKS: usize = 50;

/// Synchronizes the storage of the node by verifying and storing new data
/// (e.g., transactions and outputs).
pub trait StorageSynchronizerInterface {
    /// Applies a batch of transaction outputs.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    fn apply_transaction_outputs(
        &mut self,
        notification_id: NotificationId,
        output_list_with_proof: TransactionOutputListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Executes a batch of transactions.
    ///
    /// Note: this assumes that the ledger infos have already been verified.
    fn execute_transactions(
        &mut self,
        notification_id: NotificationId,
        transaction_list_with_proof: TransactionListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error>;

    /// Returns true iff there is transaction data that is still waiting
    /// to be executed/applied or committed.
    fn pending_transaction_data(&self) -> bool;

    /// Saves the given account states to storage
    fn save_account_states(
        &mut self,
        account_states_with_proof: AccountStatesChunkWithProof,
    ) -> Result<(), Error>;
}

/// The implementation of the `StorageSynchronizerInterface` used by state sync
#[derive(Clone)]
pub struct StorageSynchronizer {
    // A channel through which to notify the executor of new transaction data chunks
    executor_notifier: mpsc::Sender<TransactionDataChunk>,

    // The number of transaction data chunks pending execute/apply, or commit
    pending_transaction_chunks: Arc<AtomicU64>,
}

impl StorageSynchronizer {
    pub fn new<ChunkExecutor: ChunkExecutorTrait + 'static>(
        chunk_executor: Arc<ChunkExecutor>,
        commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
        error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
        runtime: Option<&Runtime>,
    ) -> Self {
        // Create a channel to notify the executor when transaction data chunks are ready
        let (executor_notifier, executor_listener) = mpsc::channel(MAX_PENDING_CHUNKS);

        // Create a channel to notify the committer when executed chunks are ready
        let (committer_notifier, committer_listener) = mpsc::channel(MAX_PENDING_CHUNKS);

        // Create a shared pending transaction chunk counter
        let pending_transaction_chunks = Arc::new(AtomicU64::new(0));

        // Spawn the executor that executes/applies transaction data chunks
        spawn_executor(
            chunk_executor.clone(),
            error_notification_sender.clone(),
            executor_listener,
            committer_notifier,
            pending_transaction_chunks.clone(),
            runtime,
        );

        // Spawn the committer that commits executed (but pending) chunks
        spawn_committer(
            chunk_executor,
            committer_listener,
            commit_notification_sender,
            error_notification_sender,
            pending_transaction_chunks.clone(),
            runtime,
        );

        Self {
            executor_notifier,
            pending_transaction_chunks,
        }
    }

    /// Notifies the executor of new transaction data chunks
    fn notify_executor(
        &mut self,
        transaction_data_chunk: TransactionDataChunk,
    ) -> Result<(), Error> {
        if let Err(error) = self.executor_notifier.try_send(transaction_data_chunk) {
            Err(Error::UnexpectedError(format!(
                "Failed to send transaction data chunk to executor: {:?}",
                error
            )))
        } else {
            increment_atomic(self.pending_transaction_chunks.clone());
            Ok(())
        }
    }
}

impl StorageSynchronizerInterface for StorageSynchronizer {
    fn apply_transaction_outputs(
        &mut self,
        notification_id: NotificationId,
        output_list_with_proof: TransactionOutputListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        let transaction_data_chunk = TransactionDataChunk::TransactionOutputs(
            notification_id,
            output_list_with_proof,
            target_ledger_info,
            end_of_epoch_ledger_info,
        );
        self.notify_executor(transaction_data_chunk)
    }

    fn execute_transactions(
        &mut self,
        notification_id: NotificationId,
        transaction_list_with_proof: TransactionListWithProof,
        target_ledger_info: LedgerInfoWithSignatures,
        end_of_epoch_ledger_info: Option<LedgerInfoWithSignatures>,
    ) -> Result<(), Error> {
        let transaction_data_chunk = TransactionDataChunk::Transactions(
            notification_id,
            transaction_list_with_proof,
            target_ledger_info,
            end_of_epoch_ledger_info,
        );
        self.notify_executor(transaction_data_chunk)
    }

    fn pending_transaction_data(&self) -> bool {
        self.pending_transaction_chunks.load(Ordering::Relaxed) > 0
    }

    fn save_account_states(
        &mut self,
        _account_states_with_proof: AccountStatesChunkWithProof,
    ) -> Result<(), Error> {
        unimplemented!("Saving account states to storage is not currently supported!")
    }
}

/// A chunk of data (i.e., transactions or transaction outputs) to be executed
/// and committed.
enum TransactionDataChunk {
    Transactions(
        NotificationId,
        TransactionListWithProof,
        LedgerInfoWithSignatures,
        Option<LedgerInfoWithSignatures>,
    ),
    TransactionOutputs(
        NotificationId,
        TransactionOutputListWithProof,
        LedgerInfoWithSignatures,
        Option<LedgerInfoWithSignatures>,
    ),
}

/// Spawns a dedicated executor that executes/applies transaction data chunks
fn spawn_executor<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    mut executor_listener: mpsc::Receiver<TransactionDataChunk>,
    mut committer_notifier: mpsc::Sender<NotificationId>,
    pending_transaction_chunks: Arc<AtomicU64>,
    runtime: Option<&Runtime>,
) {
    // Create an executor
    let executor = async move {
        loop {
            ::futures::select! {
                transaction_data_chunk = executor_listener.select_next_some() => {
                    // Execute/apply the transaction data chunk
                    let (notification_id, result) = match transaction_data_chunk {
                        TransactionDataChunk::Transactions(notification_id, transactions_with_proof, target_ledger_info, end_of_epoch_ledger_info) => {
                            let result = chunk_executor
                               .execute_chunk(
                                    transactions_with_proof,
                                    &target_ledger_info,
                                    end_of_epoch_ledger_info.as_ref(),
                                );
                            (notification_id, result)
                        },
                        TransactionDataChunk::TransactionOutputs(notification_id, outputs_with_proof, target_ledger_info, end_of_epoch_ledger_info) => {
                            let result = chunk_executor
                                .apply_chunk(
                                    outputs_with_proof,
                                    &target_ledger_info,
                                    end_of_epoch_ledger_info.as_ref(),
                                );
                             (notification_id, result)
                        }
                    };

                    // Notify the committer of new executed chunks
                    match result {
                        Ok(()) => {
                            if let Err(error) = committer_notifier.try_send(notification_id) {
                                let error = format!("Failed to notify the committer! Error: {:?}", error);
                                send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                                decrement_atomic(pending_transaction_chunks.clone());
                            }
                        },
                        Err(error) => {
                            let error = format!("Failed to execute/apply the transaction data chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                            decrement_atomic(pending_transaction_chunks.clone());
                        }
                    }
                }
            }
        }
    };

    // Spawn the executor
    spawn(runtime, executor);
}

/// Spawns a dedicated committer that commits executed (but pending) chunks
fn spawn_committer<ChunkExecutor: ChunkExecutorTrait + 'static>(
    chunk_executor: Arc<ChunkExecutor>,
    mut committer_listener: mpsc::Receiver<NotificationId>,
    mut commit_notification_sender: mpsc::UnboundedSender<CommitNotification>,
    error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    pending_transaction_chunks: Arc<AtomicU64>,
    runtime: Option<&Runtime>,
) {
    // Create an executor
    let committer = async move {
        loop {
            ::futures::select! {
                notification_id = committer_listener.select_next_some() => {
                    // Commit the executed chunk
                    match chunk_executor.commit_chunk() {
                        Ok((events, transactions)) => {
                            let commit_notification = CommitNotification::new(events, transactions);
                            if let Err(error) = commit_notification_sender.send(commit_notification).await {
                                let error = format!("Failed to send commit notification! Error: {:?}", error);
                                send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                            }
                        }
                        Err(error) => {
                            let error = format!("Failed to commit executed chunk! Error: {:?}", error);
                            send_storage_synchronizer_error(error_notification_sender.clone(), notification_id, error).await;
                        }
                    };
                    decrement_atomic(pending_transaction_chunks.clone());
                }
            }
        }
    };

    // Spawn the committer
    spawn(runtime, committer);
}

/// Spawns a future on a specified runtime. If no runtime is specified, uses
/// the current runtime.
fn spawn(runtime: Option<&Runtime>, future: impl Future<Output = ()> + Send + 'static) {
    if let Some(runtime) = runtime {
        runtime.spawn(future);
    } else {
        tokio::spawn(future);
    }
}

/// Increments the given atomic u64
fn increment_atomic(atomic_u64: Arc<AtomicU64>) {
    atomic_u64.fetch_add(1, Ordering::Relaxed);
}

/// Decrements the given atomic u64
fn decrement_atomic(atomic_u64: Arc<AtomicU64>) {
    atomic_u64.fetch_sub(1, Ordering::Relaxed);
}

/// Sends an error notification to the notification listener
async fn send_storage_synchronizer_error(
    mut error_notification_sender: mpsc::UnboundedSender<ErrorNotification>,
    notification_id: NotificationId,
    error_message: String,
) {
    let error_message = format!("Storage synchronizer error: {:?}", error_message);
    error!("{:?}", error_message);

    // Send an error notification
    let error_notification = ErrorNotification {
        error: Error::UnexpectedError(error_message),
        notification_id,
    };
    if let Err(error) = error_notification_sender.send(error_notification).await {
        panic!("Failed to send error notification! Error: {:?}", error);
    }
}
