// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    operational_tooling::{
        launch_swarm_with_op_tool_and_backend, wait_for_transaction_on_all_nodes,
    },
    smoke_test_environment::new_local_swarm,
    test_utils::{check_create_mint_transfer, diem_swarm_utils::load_validators_backend_storage},
};
use diem_config::config::SecureBackend;
use diem_global_constants::OWNER_ACCOUNT;
use diem_operational_tool::test_helper::OperationalTool;
use diem_sdk::types::on_chain_config::OnChainConsensusConfig;
use diem_secure_storage::{KVStorage, Storage};
use diem_types::{
    account_address::AccountAddress,
    network_address::NetworkAddress,
    on_chain_config::{ConsensusConfigV1, ConsensusConfigV2},
};
use forge::{LocalSwarm, Node, NodeExt, Swarm};
use std::{convert::TryInto, str::FromStr};

#[tokio::test]
async fn test_consensus_observer_mode_storage_error() {
    let num_nodes = 4;
    let (swarm, op_tool, backend, _) = launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // Kill safety rules storage for validator 1 to ensure it fails on the next epoch change
    let node_config = swarm.validators().nth(1).unwrap().config().clone();
    let safety_rules_storage = match node_config.consensus.safety_rules.backend {
        SecureBackend::OnDiskStorage(config) => SecureBackend::OnDiskStorage(config),
        _ => panic!("On-disk storage is the only backend supported in smoke tests"),
    };
    let mut safety_rules_storage: Storage = (&safety_rules_storage).try_into().unwrap();
    safety_rules_storage.reset_and_clear().unwrap();

    // Force a new epoch by updating validator 0's full node address in the validator config
    let txn_ctx = op_tool
        .set_validator_config(
            None,
            Some(NetworkAddress::from_str("/ip4/10.0.0.16/tcp/80").unwrap()),
            &backend,
            false,
            false,
        )
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Rotate validator 0's operator key several different times, each requiring a new transaction
    for _ in 0..5 {
        let (txn_ctx, _) = op_tool.rotate_operator_key(&backend, false).await.unwrap();
        assert!(txn_ctx.execution_result.unwrap().success);
    }

    // Verify validator 1 is still able to stay up to date with validator 0 (despite safety rules failing)
    let client_0 = swarm.validators().next().unwrap().rest_client();
    let sequence_number_0 = client_0
        .get_account(txn_ctx.address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number;
    let client_1 = swarm.validators().nth(1).unwrap().rest_client();
    let sequence_number_1 = client_1
        .get_account(txn_ctx.address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number;
    assert_eq!(sequence_number_0, sequence_number_1);
}

#[tokio::test]
async fn test_safety_rules_export_consensus() {
    // Create the smoke test environment
    let num_nodes = 4;
    let mut swarm = new_local_swarm(num_nodes).await;

    // Update all nodes to export the consensus key
    for validator in swarm.validators_mut() {
        let mut node_config = validator.config().clone();
        node_config.consensus.safety_rules.export_consensus_key = true;
        node_config.save(validator.config_path()).unwrap();
        validator.restart().await.unwrap();
    }

    // Launch and test the swarm
    swarm.launch().await.unwrap();
    rotate_operator_and_consensus_key(swarm).await;
}

#[tokio::test]
async fn test_safety_rules_export_consensus_compatibility() {
    // Create the smoke test environment
    let num_nodes = 4;
    let mut swarm = new_local_swarm(num_nodes).await;

    // Allow the first and second nodes to export the consensus key
    for validator in swarm.validators_mut().take(2) {
        let mut node_config = validator.config().clone();
        node_config.consensus.safety_rules.export_consensus_key = true;
        node_config.save(validator.config_path()).unwrap();
        validator.restart().await.unwrap();
    }

    // Launch and test the swarm
    swarm.launch().await.unwrap();
    rotate_operator_and_consensus_key(swarm).await;
}

#[tokio::test]
async fn test_2chain_upgrade() {
    // genesis starts with 2-chain already
    test_onchain_upgrade(OnChainConsensusConfig::V1(ConsensusConfigV1 {
        two_chain: false,
    }))
    .await;
}

#[tokio::test]
async fn test_decoupled_execution_upgrade() {
    test_onchain_upgrade(OnChainConsensusConfig::V2(ConsensusConfigV2 {
        two_chain: true,
        decoupled_execution: true,
        back_pressure_limit: 10,
        exclude_round: 20,
    }))
    .await
}

async fn rotate_operator_and_consensus_key(swarm: LocalSwarm) {
    let validator = swarm.validators().next().unwrap();
    let rest_api_endpoint = validator.rest_api_endpoint().to_string();

    // Load the first validator's on disk storage
    let backend = load_validators_backend_storage(validator);
    let storage: Storage = (&backend).try_into().unwrap();

    // Connect the operator tool to the first node's JSON RPC API
    let op_tool = OperationalTool::new(rest_api_endpoint, swarm.chain_id());

    // Rotate the first node's operator key
    let (txn_ctx, _) = op_tool.rotate_operator_key(&backend, true).await.unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Ensure all nodes have received the transaction
    wait_for_transaction_on_all_nodes(&swarm, txn_ctx.address, txn_ctx.sequence_number).await;

    // Rotate the consensus key to verify the operator key has been updated
    let (txn_ctx, new_consensus_key) = op_tool.rotate_consensus_key(&backend, false).await.unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Ensure all nodes have received the transaction
    wait_for_transaction_on_all_nodes(&swarm, txn_ctx.address, txn_ctx.sequence_number).await;

    // Verify that the config has been updated correctly with the new consensus key
    let validator_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let config_consensus_key = op_tool
        .validator_config(validator_account, Some(&backend))
        .await
        .unwrap()
        .consensus_public_key;
    assert_eq!(new_consensus_key, config_consensus_key);
}

async fn test_onchain_upgrade(new_onfig: OnChainConsensusConfig) {
    let num_nodes = 4;
    let (mut swarm, _, _, _) = launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // should work before upgrade.
    check_create_mint_transfer(&mut swarm).await;

    // send upgrade txn
    let transaction_factory = swarm.chain_info().transaction_factory();
    let upgrade_txn = swarm
        .chain_info()
        .root_account
        .sign_with_transaction_builder(
            transaction_factory.update_diem_consensus_config(0, bcs::to_bytes(&new_onfig).unwrap()),
        );

    let client = swarm.validators().next().unwrap().rest_client();
    client.submit_and_wait(&upgrade_txn).await.unwrap();

    // should work after upgrade.
    check_create_mint_transfer(&mut swarm).await;
}
