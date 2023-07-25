// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    handler::Handler,
    metrics,
    metrics::{increment_counter, OPTIMISTIC_FETCH_EXPIRE},
    moderator::RequestModerator,
    network::ResponseSender,
    storage::StorageReaderInterface,
    LogEntry, LogSchema,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, StorageServiceRequest,
        TransactionOutputsWithProofRequest, TransactionsOrOutputsWithProofRequest,
        TransactionsWithProofRequest,
    },
    responses::{DataResponse, StorageServerSummary, StorageServiceResponse},
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use lru::LruCache;
use std::{cmp::min, sync::Arc, time::Instant};

/// An optimistic fetch request from a peer
pub struct OptimisticFetchRequest {
    request: StorageServiceRequest,
    response_sender: ResponseSender,
    fetch_start_time: Instant,
    time_service: TimeService,
}

impl OptimisticFetchRequest {
    pub fn new(
        request: StorageServiceRequest,
        response_sender: ResponseSender,
        time_service: TimeService,
    ) -> Self {
        Self {
            request,
            response_sender,
            fetch_start_time: time_service.now(),
            time_service,
        }
    }

    /// Creates a new storage service request to satisfy the optimistic fetch
    /// using the new data at the specified `target_ledger_info`.
    fn get_storage_request_for_missing_data(
        &self,
        config: StorageServiceConfig,
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> aptos_storage_service_types::Result<StorageServiceRequest, Error> {
        // Calculate the number of versions to fetch
        let known_version = self.highest_known_version();
        let target_version = target_ledger_info.ledger_info().version();
        let mut num_versions_to_fetch =
            target_version.checked_sub(known_version).ok_or_else(|| {
                Error::UnexpectedErrorEncountered(
                    "Number of versions to fetch has overflown!".into(),
                )
            })?;

        // Bound the number of versions to fetch by the maximum chunk size
        num_versions_to_fetch = min(
            num_versions_to_fetch,
            self.max_chunk_size_for_request(config),
        );

        // Calculate the start and end versions
        let start_version = known_version.checked_add(1).ok_or_else(|| {
            Error::UnexpectedErrorEncountered("Start version has overflown!".into())
        })?;
        let end_version = known_version
            .checked_add(num_versions_to_fetch)
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered("End version has overflown!".into())
            })?;

        // Create the storage request
        let data_request = match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(_) => {
                DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                    proof_version: target_version,
                    start_version,
                    end_version,
                })
            },
            DataRequest::GetNewTransactionsWithProof(request) => {
                DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version: target_version,
                    start_version,
                    end_version,
                    include_events: request.include_events,
                })
            },
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => {
                DataRequest::GetTransactionsOrOutputsWithProof(
                    TransactionsOrOutputsWithProofRequest {
                        proof_version: target_version,
                        start_version,
                        end_version,
                        include_events: request.include_events,
                        max_num_output_reductions: request.max_num_output_reductions,
                    },
                )
            },
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        };
        let storage_request =
            StorageServiceRequest::new(data_request, self.request.use_compression);
        Ok(storage_request)
    }

    /// Returns the highest version known by the peer
    fn highest_known_version(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(request) => request.known_version,
            DataRequest::GetNewTransactionsWithProof(request) => request.known_version,
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => request.known_version,
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        }
    }

    /// Returns the highest epoch known by the peer
    fn highest_known_epoch(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => request.known_epoch,
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        }
    }

    /// Returns the maximum chunk size for the request depending
    /// on the request type.
    fn max_chunk_size_for_request(&self, config: StorageServiceConfig) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(_) => {
                config.max_transaction_output_chunk_size
            },
            DataRequest::GetNewTransactionsWithProof(_) => config.max_transaction_chunk_size,
            DataRequest::GetNewTransactionsOrOutputsWithProof(_) => {
                config.max_transaction_output_chunk_size
            },
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        }
    }

    /// Returns true iff the optimistic fetch has expired
    fn is_expired(&self, timeout_ms: u64) -> bool {
        let current_time = self.time_service.now();
        let elapsed_time = current_time
            .duration_since(self.fetch_start_time)
            .as_millis();
        elapsed_time > timeout_ms as u128
    }
}

/// Handles ready optimistic fetches
pub(crate) async fn handle_active_optimistic_fetches<T: StorageReaderInterface>(
    bounded_executor: BoundedExecutor,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
) -> Result<(), Error> {
    // Identify the peers with ready optimistic fetches
    let peers_with_ready_optimistic_fetches = get_peers_with_ready_optimistic_fetches(
        config,
        cached_storage_server_summary.clone(),
        optimistic_fetches.clone(),
        lru_response_cache.clone(),
        request_moderator.clone(),
        storage.clone(),
        time_service.clone(),
    )?;

    // Remove and handle the ready optimistic fetches
    handle_ready_optimistic_fetches(
        bounded_executor,
        cached_storage_server_summary,
        config,
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        time_service,
        peers_with_ready_optimistic_fetches,
    )
    .await;

    Ok(())
}

/// Handles the ready optimistic fetches by removing them from the
/// active map and notifying the peer of the new data.
async fn handle_ready_optimistic_fetches<T: StorageReaderInterface>(
    bounded_executor: BoundedExecutor,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    peers_with_ready_optimistic_fetches: Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
) {
    for (peer_network_id, target_ledger_info) in peers_with_ready_optimistic_fetches {
        // Remove the optimistic fetch from the active map
        let ready_optimistic_fetch = optimistic_fetches.clone().remove(&peer_network_id);

        // Handle the optimistic fetch request
        if let Some((_, optimistic_fetch)) = ready_optimistic_fetch {
            // Clone all required components for the task
            let cached_storage_server_summary = cached_storage_server_summary.clone();
            let optimistic_fetches = optimistic_fetches.clone();
            let lru_response_cache = lru_response_cache.clone();
            let request_moderator = request_moderator.clone();
            let storage = storage.clone();
            let time_service = time_service.clone();

            // Spawn a blocking task to handle the optimistic fetch
            bounded_executor
                .spawn_blocking(move || {
                    // Get the fetch start time and request
                    let optimistic_fetch_start_time = optimistic_fetch.fetch_start_time;
                    let optimistic_fetch_request = optimistic_fetch.request.clone();

                    // Notify the peer of the new data
                    if let Err(error) = notify_peer_of_new_data(
                        cached_storage_server_summary.clone(),
                        config,
                        optimistic_fetches.clone(),
                        lru_response_cache.clone(),
                        request_moderator.clone(),
                        storage.clone(),
                        time_service.clone(),
                        &peer_network_id,
                        optimistic_fetch,
                        target_ledger_info,
                    ) {
                        warn!(LogSchema::new(LogEntry::OptimisticFetchResponse)
                            .error(&Error::UnexpectedErrorEncountered(error.to_string())));
                    }

                    // Update the optimistic fetch latency metric
                    let optimistic_fetch_duration = time_service
                        .now()
                        .duration_since(optimistic_fetch_start_time);
                    metrics::observe_value_with_label(
                        &metrics::OPTIMISTIC_FETCH_LATENCIES,
                        peer_network_id.network_id(),
                        &optimistic_fetch_request.get_label(),
                        optimistic_fetch_duration.as_secs_f64(),
                    );
                })
                .await;
        }
    }
}

/// Identifies the optimistic fetches that can be handled now.
/// Returns the list of peers that made those optimistic fetches
/// alongside the ledger info at the target version for the peer.
pub(crate) fn get_peers_with_ready_optimistic_fetches<T: StorageReaderInterface>(
    config: StorageServiceConfig,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
) -> aptos_storage_service_types::Result<Vec<(PeerNetworkId, LedgerInfoWithSignatures)>, Error> {
    // Fetch the latest storage summary and highest synced version
    let latest_storage_summary = cached_storage_server_summary.load().clone();
    let highest_synced_ledger_info = match &latest_storage_summary.data_summary.synced_ledger_info {
        Some(ledger_info) => ledger_info.clone(),
        None => return Ok(vec![]),
    };
    let highest_synced_version = highest_synced_ledger_info.ledger_info().version();
    let highest_synced_epoch = highest_synced_ledger_info.ledger_info().epoch();

    // Identify the peers with expired, invalid and ready optimistic fetches
    let mut peers_with_expired_optimistic_fetches = vec![];
    let mut peers_with_invalid_optimistic_fetches = vec![];
    let mut peers_with_ready_optimistic_fetches = vec![];
    for optimistic_fetch in optimistic_fetches.iter() {
        // Get the peer and the optimistic fetch request
        let peer = optimistic_fetch.key();
        let optimistic_fetch = optimistic_fetch.value();

        // Ensure the optimistic fetch hasn't expired
        if optimistic_fetch.is_expired(config.max_optimistic_fetch_period) {
            peers_with_expired_optimistic_fetches.push(*peer);
            continue;
        }

        // Check if we have synced beyond the highest known version
        let highest_known_version = optimistic_fetch.highest_known_version();
        if highest_known_version < highest_synced_version {
            let highest_known_epoch = optimistic_fetch.highest_known_epoch();
            if highest_known_epoch < highest_synced_epoch {
                // The peer needs to sync to their epoch ending ledger info
                let epoch_ending_ledger_info = get_epoch_ending_ledger_info(
                    cached_storage_server_summary.clone(),
                    optimistic_fetches.clone(),
                    highest_known_epoch,
                    lru_response_cache.clone(),
                    request_moderator.clone(),
                    peer,
                    storage.clone(),
                    time_service.clone(),
                )?;

                // Check that we haven't been sent an invalid optimistic fetch request
                // (i.e., a request that does not respect an epoch boundary).
                if epoch_ending_ledger_info.ledger_info().version() <= highest_known_version {
                    peers_with_invalid_optimistic_fetches.push(*peer);
                } else {
                    peers_with_ready_optimistic_fetches.push((*peer, epoch_ending_ledger_info));
                }
            } else {
                peers_with_ready_optimistic_fetches
                    .push((*peer, highest_synced_ledger_info.clone()));
            };
        }
    }

    // Remove the expired optimistic fetches
    removed_expired_optimistic_fetches(
        optimistic_fetches.clone(),
        peers_with_expired_optimistic_fetches,
    );

    // Remove the invalid optimistic fetches
    remove_invalid_optimistic_fetches(optimistic_fetches, peers_with_invalid_optimistic_fetches);

    // Return the ready optimistic fetches
    Ok(peers_with_ready_optimistic_fetches)
}

/// Gets the epoch ending ledger info at the given epoch
fn get_epoch_ending_ledger_info<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    epoch: u64,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    peer_network_id: &PeerNetworkId,
    storage: T,
    time_service: TimeService,
) -> aptos_storage_service_types::Result<LedgerInfoWithSignatures, Error> {
    // Create a new storage request for the epoch ending ledger info
    let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch: epoch,
        expected_end_epoch: epoch,
    });
    let storage_request = StorageServiceRequest::new(
        data_request,
        false, // Don't compress because this isn't going over the wire
    );

    // Process the request
    let handler = Handler::new(
        cached_storage_server_summary,
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        time_service,
    );
    let storage_response = handler.process_request(peer_network_id, storage_request, true);

    // Verify the response
    match storage_response {
        Ok(storage_response) => match &storage_response.get_data_response() {
            Ok(DataResponse::EpochEndingLedgerInfos(epoch_change_proof)) => {
                if let Some(ledger_info) = epoch_change_proof.ledger_info_with_sigs.first() {
                    Ok(ledger_info.clone())
                } else {
                    Err(Error::UnexpectedErrorEncountered(
                        "Empty change proof found!".into(),
                    ))
                }
            },
            data_response => Err(Error::StorageErrorEncountered(format!(
                "Failed to get epoch ending ledger info! Got: {:?}",
                data_response
            ))),
        },
        Err(error) => Err(Error::StorageErrorEncountered(format!(
            "Failed to get epoch ending ledger info! Error: {:?}",
            error
        ))),
    }
}

/// Notifies a peer of new data according to the target ledger info.
///
/// Note: we don't need to check the size of the optimistic fetch response
/// because: (i) each sub-part should already be checked; and (ii)
/// optimistic fetch responses are best effort.
fn notify_peer_of_new_data<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    peer_network_id: &PeerNetworkId,
    optimistic_fetch: OptimisticFetchRequest,
    target_ledger_info: LedgerInfoWithSignatures,
) -> aptos_storage_service_types::Result<(), Error> {
    match optimistic_fetch.get_storage_request_for_missing_data(config, &target_ledger_info) {
        Ok(storage_request) => {
            // Handle the storage service request to fetch the missing data
            let use_compression = storage_request.use_compression;
            let handler = Handler::new(
                cached_storage_server_summary,
                optimistic_fetches,
                lru_response_cache,
                request_moderator,
                storage,
                time_service,
            );
            let storage_response =
                handler.process_request(peer_network_id, storage_request.clone(), true);

            // Transform the missing data into an optimistic fetch response
            let transformed_data_response = match storage_response {
                Ok(storage_response) => match storage_response.get_data_response() {
                    Ok(DataResponse::TransactionsWithProof(transactions_with_proof)) => {
                        DataResponse::NewTransactionsWithProof((
                            transactions_with_proof,
                            target_ledger_info.clone(),
                        ))
                    },
                    Ok(DataResponse::TransactionOutputsWithProof(outputs_with_proof)) => {
                        DataResponse::NewTransactionOutputsWithProof((
                            outputs_with_proof,
                            target_ledger_info.clone(),
                        ))
                    },
                    Ok(DataResponse::TransactionsOrOutputsWithProof((
                        transactions_with_proof,
                        outputs_with_proof,
                    ))) => {
                        if let Some(transactions_with_proof) = transactions_with_proof {
                            DataResponse::NewTransactionsOrOutputsWithProof((
                                (Some(transactions_with_proof), None),
                                target_ledger_info.clone(),
                            ))
                        } else if let Some(outputs_with_proof) = outputs_with_proof {
                            DataResponse::NewTransactionsOrOutputsWithProof((
                                (None, Some(outputs_with_proof)),
                                target_ledger_info.clone(),
                            ))
                        } else {
                            return Err(Error::UnexpectedErrorEncountered(
                                "Failed to get a transaction or output response for peer!".into(),
                            ));
                        }
                    },
                    data_response => {
                        return Err(Error::UnexpectedErrorEncountered(format!(
                            "Failed to get appropriate data response for peer! Got: {:?}",
                            data_response
                        )))
                    },
                },
                response => {
                    return Err(Error::UnexpectedErrorEncountered(format!(
                        "Failed to fetch missing data for peer! {:?}",
                        response
                    )))
                },
            };
            let storage_response =
                match StorageServiceResponse::new(transformed_data_response, use_compression) {
                    Ok(storage_response) => storage_response,
                    Err(error) => {
                        return Err(Error::UnexpectedErrorEncountered(format!(
                            "Failed to create transformed response! Error: {:?}",
                            error
                        )));
                    },
                };

            // Send the response to the peer
            handler.send_response(
                storage_request,
                Ok(storage_response),
                optimistic_fetch.response_sender,
            );
            Ok(())
        },
        Err(error) => Err(error),
    }
}

/// Removes the expired optimistic fetches from the active map
fn removed_expired_optimistic_fetches(
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    peers_with_expired_optimistic_fetches: Vec<PeerNetworkId>,
) {
    for peer_network_id in peers_with_expired_optimistic_fetches {
        if optimistic_fetches.remove(&peer_network_id).is_some() {
            increment_counter(
                &metrics::OPTIMISTIC_FETCH_EVENTS,
                peer_network_id.network_id(),
                OPTIMISTIC_FETCH_EXPIRE.into(),
            );
        }
    }
}

/// Removes the invalid optimistic fetches from the active map
fn remove_invalid_optimistic_fetches(
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    peers_with_invalid_optimistic_fetches: Vec<PeerNetworkId>,
) {
    for peer_network_id in peers_with_invalid_optimistic_fetches {
        if let Some((_, optimistic_fetch)) = optimistic_fetches.remove(&peer_network_id) {
            warn!(LogSchema::new(LogEntry::OptimisticFetchRefresh)
                .error(&Error::InvalidRequest(
                    "Mismatch between known version and epoch!".into()
                ))
                .request(&optimistic_fetch.request)
                .message("Dropping invalid optimistic fetch request!"));
        }
    }
}
