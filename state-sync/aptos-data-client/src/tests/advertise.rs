// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    error::Error,
    interface::AptosDataClientInterface,
    peer_states::calculate_optimal_chunk_sizes,
    poller,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::{config::AptosDataClientConfig, network_id::PeerNetworkId};
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionsWithProofRequest},
    responses::{CompleteDataRange, DataResponse, StorageServerSummary, StorageServiceResponse},
};
use aptos_types::transaction::{TransactionListWithProof, Version};
use claims::assert_matches;

#[tokio::test]
async fn request_works_only_when_data_available() {
    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network, mock time, client and poller
        let data_client_config = AptosDataClientConfig::default();
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Request transactions and verify the request fails (no peers are connected)
        fetch_transactions_and_verify_failure(&data_client_config, &client, 100).await;

        // Add a connected peer
        let (peer, network_id) = utils::add_peer_to_network(poll_priority_peers, &mut mock_network);

        // Verify the peer's state has not been updated
        let peer_states = client.get_peer_states();
        let peer_to_states = peer_states.get_peer_to_states();
        assert!(peer_to_states.is_empty());

        // Request transactions and verify the request fails (no peers are advertising data)
        fetch_transactions_and_verify_failure(&data_client_config, &client, 100).await;

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Get and verify the received network request
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        assert_eq!(network_request.peer_network_id, peer);

        // Handle the request
        let storage_summary = utils::create_storage_summary(200);
        utils::handle_storage_summary_request(network_request, storage_summary.clone());

        // Let the poller finish processing the response
        tokio::task::yield_now().await;

        // Handle the client's transaction request
        tokio::spawn(async move {
            // Verify the received network request
            let network_request = utils::get_network_request(&mut mock_network, network_id).await;
            assert_matches!(
                network_request.storage_service_request.data_request,
                DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    start_version: 0,
                    end_version: 100,
                    proof_version: 100,
                    include_events: false,
                })
            );

            // Fulfill the request
            utils::handle_transactions_request(network_request, true);
        });

        // Verify the peer's state has been updated
        let peer_state = peer_to_states.get(&peer).unwrap().value().clone();
        let peer_storage_summary = peer_state
            .get_storage_summary_if_not_ignored()
            .unwrap()
            .clone();
        assert_eq!(peer_storage_summary, storage_summary);

        // Request transactions and verify the request succeeds
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 0, 100, false, request_timeout)
            .await
            .unwrap();
        assert_eq!(response.payload, TransactionListWithProof::new_empty());
    }
}

#[tokio::test]
async fn update_global_data_summary() {
    // Create the mock network, mock time, client and poller
    let data_client_config = AptosDataClientConfig::default();
    let (mut mock_network, mut mock_time, client, poller) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Start the poller
    tokio::spawn(poller::start_poller(poller));

    // Verify the global data summary is empty
    let global_data_summary = client.get_global_data_summary();
    assert!(global_data_summary.is_empty());

    // Add a priority peer
    let (_, priority_network) = utils::add_peer_to_network(true, &mut mock_network);

    // Advance time so the poller sends a data summary request to the peer
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the priority peer's data summary request
    let network_request = utils::get_network_request(&mut mock_network, priority_network).await;
    let priority_peer_version = 10_000;
    let priority_storage_summary = utils::create_storage_summary(priority_peer_version);
    let data_response = DataResponse::StorageServerSummary(priority_storage_summary.clone());
    network_request
        .response_sender
        .send(Ok(StorageServiceResponse::new(data_response, true).unwrap()));

    // Advance time so the poller updates the global data summary
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Verify that the advertised data ranges are valid
    verify_advertised_transaction_data(&client, priority_peer_version, 1, true);

    // Add a regular peer
    let (_, regular_network) = utils::add_peer_to_network(false, &mut mock_network);

    // Advance time so the poller sends a data summary request for both peers
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the priority peer's data summary request
    let network_request = utils::get_network_request(&mut mock_network, priority_network).await;
    let priority_peer_version = 20_000;
    let priority_storage_summary = utils::create_storage_summary(priority_peer_version);
    utils::handle_storage_summary_request(network_request, priority_storage_summary.clone());

    // Handle the regular peer's data summary request (using more data)
    let network_request = utils::get_network_request(&mut mock_network, regular_network).await;
    let regular_peer_version = 30_000;
    let regular_storage_summary = utils::create_storage_summary(regular_peer_version);
    utils::handle_storage_summary_request(network_request, regular_storage_summary.clone());

    // Advance time so the poller elapses
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Verify that the advertised data ranges are valid
    verify_advertised_transaction_data(&client, priority_peer_version, 2, false);
    verify_advertised_transaction_data(&client, regular_peer_version, 2, true);
}

#[tokio::test]
async fn update_peer_states() {
    // Create the mock network, mock time, client and poller
    let data_client_config = AptosDataClientConfig::default();
    let (mut mock_network, mut mock_time, client, poller) =
        MockNetwork::new(None, Some(data_client_config), None);

    // Start the poller
    tokio::spawn(poller::start_poller(poller));

    // Add a priority peer
    let (priority_peer, priority_network) = utils::add_peer_to_network(true, &mut mock_network);

    // Verify that we have no peer states
    let peer_states = client.get_peer_states();
    let peer_to_states = peer_states.get_peer_to_states();
    assert!(peer_to_states.is_empty());

    // Advance time so the poller sends a data summary request for the peer
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the priority peer's data summary request
    let network_request = utils::get_network_request(&mut mock_network, priority_network).await;
    let priority_storage_summary = utils::create_storage_summary(1111);
    utils::handle_storage_summary_request(network_request, priority_storage_summary.clone());

    // Let the poller finish processing the responses
    tokio::task::yield_now().await;

    // Verify that the priority peer's state has been updated
    verify_peer_state(&client, priority_peer, priority_storage_summary);

    // Add a regular peer
    let (regular_peer, regular_network) = utils::add_peer_to_network(false, &mut mock_network);

    // Advance time so the poller sends a data summary request for both peers
    utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

    // Handle the priority peer's data summary request
    let network_request = utils::get_network_request(&mut mock_network, priority_network).await;
    let priority_storage_summary = utils::create_storage_summary(2222);
    utils::handle_storage_summary_request(network_request, priority_storage_summary.clone());

    // Handle the regular peer's data summary request
    let network_request = utils::get_network_request(&mut mock_network, regular_network).await;
    let regular_storage_summary = utils::create_storage_summary(3333);
    utils::handle_storage_summary_request(network_request, regular_storage_summary.clone());

    // Let the poller finish processing the responses
    tokio::task::yield_now().await;

    // Verify that the priority peer's state has been updated
    verify_peer_state(&client, priority_peer, priority_storage_summary);

    // Verify that the regular peer's state has been set
    verify_peer_state(&client, regular_peer, regular_storage_summary);
}

#[tokio::test]
async fn optimal_chunk_size_calculations() {
    // Create a test storage service config
    let max_epoch_chunk_size = 600;
    let max_state_chunk_size = 500;
    let max_transaction_chunk_size = 700;
    let max_transaction_output_chunk_size = 800;
    let data_client_config = AptosDataClientConfig {
        max_epoch_chunk_size,
        max_state_chunk_size,
        max_transaction_chunk_size,
        max_transaction_output_chunk_size,
        ..Default::default()
    };

    // Test median calculations
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &data_client_config,
        vec![7, 5, 6, 8, 10],
        vec![100, 200, 300, 100],
        vec![900, 700, 500],
        vec![40],
    );
    assert_eq!(200, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(7, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(700, optimal_chunk_sizes.transaction_chunk_size);
    assert_eq!(40, optimal_chunk_sizes.transaction_output_chunk_size);

    // Test no advertised data
    let optimal_chunk_sizes =
        calculate_optimal_chunk_sizes(&data_client_config, vec![], vec![], vec![], vec![]);
    assert_eq!(max_state_chunk_size, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(max_epoch_chunk_size, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(
        max_transaction_chunk_size,
        optimal_chunk_sizes.transaction_chunk_size
    );
    assert_eq!(
        max_transaction_output_chunk_size,
        optimal_chunk_sizes.transaction_output_chunk_size
    );

    // Verify the config caps the amount of chunks
    let optimal_chunk_sizes = calculate_optimal_chunk_sizes(
        &data_client_config,
        vec![70, 50, 60, 80, 100],
        vec![1000, 1000, 2000, 3000],
        vec![9000, 7000, 5000],
        vec![400],
    );
    assert_eq!(max_state_chunk_size, optimal_chunk_sizes.state_chunk_size);
    assert_eq!(70, optimal_chunk_sizes.epoch_chunk_size);
    assert_eq!(
        max_transaction_chunk_size,
        optimal_chunk_sizes.transaction_chunk_size
    );
    assert_eq!(400, optimal_chunk_sizes.transaction_output_chunk_size);
}

/// Requests transactions up to the specified version and verifies the request fails
async fn fetch_transactions_and_verify_failure(
    data_client_config: &AptosDataClientConfig,
    data_client: &AptosDataClient,
    version: u64,
) {
    // Request the transactions with proof
    let request_timeout = data_client_config.response_timeout_ms;
    let error = data_client
        .get_transactions_with_proof(version, 0, version, false, request_timeout)
        .await
        .unwrap_err();

    // Verify the error is correct
    assert_matches!(error, Error::DataIsUnavailable(_));
}

/// Verifies that the advertised transaction data is valid
fn verify_advertised_transaction_data(
    client: &AptosDataClient,
    advertised_version: Version,
    expected_num_advertisements: usize,
    is_highest_version: bool,
) {
    // Get the advertised data
    let global_data_summary = client.get_global_data_summary();
    let advertised_data = global_data_summary.advertised_data;

    // Verify the number of advertised entries
    assert_eq!(
        advertised_data.transactions.len(),
        expected_num_advertisements
    );

    // Verify that the advertised transaction data contains an entry for the given version
    assert!(advertised_data
        .transactions
        .contains(&CompleteDataRange::new(0, advertised_version).unwrap()));

    // Verify that the highest synced ledger info is valid (if this is the highest advertised version)
    if is_highest_version {
        let highest_synced_ledger_info = advertised_data.highest_synced_ledger_info().unwrap();
        assert_eq!(
            highest_synced_ledger_info.ledger_info().version(),
            advertised_version
        );
    }
}

/// Verifies that the peer's state is valid (i.e., the storage summary is correct)
fn verify_peer_state(
    client: &AptosDataClient,
    peer: PeerNetworkId,
    expected_storage_summary: StorageServerSummary,
) {
    // Get the peer's state
    let peer_to_states = client.get_peer_states().get_peer_to_states();
    let peer_state = peer_to_states.get(&peer).unwrap().value().clone();

    // Verify that the peer's storage summary is valid
    let peer_storage_summary = peer_state
        .get_storage_summary_if_not_ignored()
        .unwrap()
        .clone();
    assert_eq!(peer_storage_summary, expected_storage_summary);
}
