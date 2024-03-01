// Copyright © Aptos Foundation

use crate::{
    randomness::{decrypt_key_map, get_on_chain_resource, verify_dkg_transcript},
    smoke_test_environment::SwarmBuilder,
    utils::get_current_consensus_config,
};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{
    dkg::DKGState,
    on_chain_config::{FeatureFlag, Features},
};
use std::{sync::Arc, time::Duration};

/// Enable on-chain randomness by enabling validator transactions and feature `RECONFIGURE_WITH_DKG` simultaneously.
#[tokio::test]
async fn enable_feature_2() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // start with vtxn disabled.
            conf.consensus_config.disable_validator_txns();

            // start with dkg disabled.
            let mut features = Features::default();
            features.disable(FeatureFlag::RECONFIGURE_WITH_DKG);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    info!("Now in epoch 3. Enabling features.");
    let mut config = get_current_consensus_config(&client).await;
    config.enable_validator_txns();
    let config_bytes = bcs::to_bytes(&config).unwrap();
    let script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;
    use std::features;
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let config_bytes = vector{:?};
        consensus_config::set_for_next_epoch(&framework_signer, config_bytes);
        let dkg_feature_id: u64 = features::get_reconfigure_with_dkg_feature();
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[dkg_feature_id], vector[]);
        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        config_bytes
    );

    debug!("script={}", script);
    let txn_summary = cli
        .run_script(root_idx, script.as_str())
        .await
        .expect("Txn execution error.");
    debug!("txn_summary={:?}", txn_summary);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    info!("Now in epoch 4. Both DKG and vtxn are enabled. There should be no randomness since DKG did not happen at the end of last epoch.");
    let maybe_last_complete = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed;
    assert!(
        maybe_last_complete.is_none() || maybe_last_complete.as_ref().unwrap().target_epoch() != 4
    );

    info!("Waiting for epoch 5.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            5,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Waited too long for epoch 5.");

    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 6 should be present");
    assert_eq!(5, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
}
