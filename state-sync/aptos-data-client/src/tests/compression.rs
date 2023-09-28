// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    interface::AptosDataClientInterface,
    poller,
    tests::{mock::MockNetwork, utils},
};
use aptos_config::config::AptosDataClientConfig;
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_storage_service_types::{
    requests::{DataRequest, TransactionsWithProofRequest},
    responses::{DataResponse, StorageServiceResponse},
};
use aptos_types::transaction::TransactionListWithProof;
use claims::assert_matches;

#[tokio::test]
async fn compression_mismatch_disabled() {
    // Create a data client config that disables compression
    let data_client_config = AptosDataClientConfig {
        use_compression: false,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(poll_priority_peers, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Receive their request and respond
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        let data_response = DataResponse::StorageServerSummary(utils::create_storage_summary(200));
        network_request.response_sender.send(Ok(
            StorageServiceResponse::new(data_response, false).unwrap()
        ));

        // Let the poller finish processing the response
        tokio::task::yield_now().await;

        // Handle the client's transactions request using compression
        tokio::spawn(async move {
            let network_request = utils::get_network_request(&mut mock_network, network_id).await;
            assert!(!network_request.storage_service_request.use_compression);

            // Compress the response
            utils::handle_transactions_request(network_request, true);
        });

        // The client should receive a compressed response and return an error
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap_err();
        assert_matches!(response, Error::InvalidResponse(_));
    }
}

#[tokio::test]
async fn compression_mismatch_enabled() {
    // Create a data client config that enables compression
    let data_client_config = AptosDataClientConfig {
        use_compression: true,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Add a connected peer
        let (_, network_id) = utils::add_peer_to_network(poll_priority_peers, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Receive their request and respond
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        utils::handle_storage_summary_request(network_request, utils::create_storage_summary(200));

        // Let the poller finish processing the response
        tokio::task::yield_now().await;

        // Handle the client's transactions request without compression
        tokio::spawn(async move {
            let network_request = utils::get_network_request(&mut mock_network, network_id).await;
            assert!(network_request.storage_service_request.use_compression);

            // Don't compress the response
            utils::handle_transactions_request(network_request, false);
        });

        // The client should receive a compressed response and return an error
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap_err();
        assert_matches!(response, Error::InvalidResponse(_));
    }
}

#[tokio::test]
async fn disable_compression() {
    // Create a data client config that disables compression
    let data_client_config = AptosDataClientConfig {
        use_compression: false,
        ..Default::default()
    };

    // Ensure the properties hold for both priority and non-priority peers
    for poll_priority_peers in [true, false] {
        // Create the mock network, mock time, client and poller
        let (mut mock_network, mut mock_time, client, poller) =
            MockNetwork::new(None, Some(data_client_config), None);

        // Start the poller
        tokio::spawn(poller::start_poller(poller));

        // Add a connected peer
        let (peer, network_id) = utils::add_peer_to_network(poll_priority_peers, &mut mock_network);

        // Advance time so the poller sends a data summary request
        utils::advance_polling_timer(&mut mock_time, &data_client_config).await;

        // Verify the received network request
        let network_request = utils::get_network_request(&mut mock_network, network_id).await;
        assert_eq!(network_request.peer_network_id, peer);
        assert_eq!(network_request.protocol_id, ProtocolId::StorageServiceRpc);
        assert!(!network_request.storage_service_request.use_compression);
        assert_matches!(
            network_request.storage_service_request.data_request,
            DataRequest::GetStorageServerSummary
        );

        // Fulfill their request
        let data_response = DataResponse::StorageServerSummary(utils::create_storage_summary(200));
        network_request.response_sender.send(Ok(
            StorageServiceResponse::new(data_response, false).unwrap()
        ));

        // Let the poller finish processing the response
        tokio::task::yield_now().await;

        // Handle the client's transactions request
        tokio::spawn(async move {
            // Verify the received network request
            let network_request = utils::get_network_request(&mut mock_network, network_id).await;
            assert_eq!(network_request.peer_network_id, peer);
            assert_eq!(network_request.protocol_id, ProtocolId::StorageServiceRpc);
            assert!(!network_request.storage_service_request.use_compression);
            assert_matches!(
                network_request.storage_service_request.data_request,
                DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    start_version: 50,
                    end_version: 100,
                    proof_version: 100,
                    include_events: false,
                })
            );

            // Fulfill the request
            let data_response =
                DataResponse::TransactionsWithProof(TransactionListWithProof::new_empty());
            let storage_response = StorageServiceResponse::new(data_response, false).unwrap();
            network_request.response_sender.send(Ok(storage_response));
        });

        // The client's request should succeed since a peer finally has advertised
        // data for this range.
        let request_timeout = data_client_config.response_timeout_ms;
        let response = client
            .get_transactions_with_proof(100, 50, 100, false, request_timeout)
            .await
            .unwrap();
        assert_eq!(response.payload, TransactionListWithProof::new_empty());
    }
}
