// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use aptos_crypto::{ed25519::Ed25519PrivateKey, secp256k1_ecdsa};
use aptos_sdk::types::{transaction::authenticator::AuthenticationKey, LocalAccount};
use rand::{rngs::StdRng, SeedableRng};
use std::convert::TryInto;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_secp256k1_ecdsa() {
    let mut context = new_test_context(current_function_name!());
    let other = context.create_account().await;

    let mut rng: StdRng = SeedableRng::from_seed([0; 32]);
    let private_key: secp256k1_ecdsa::PrivateKey = aptos_crypto::Uniform::generate(&mut rng);
    let public_key = aptos_crypto::PrivateKey::public_key(&private_key);
    let address = AuthenticationKey::secp256k1_ecdsa(&public_key).account_address();

    // Set a dummy key
    let key_bytes =
        hex::decode("a38ba78b1a0fbfc55e2c5dfdedf48d1172283d0f7c59fd64c02d811130a2f4b2").unwrap();
    let ed25519_private_key: Ed25519PrivateKey = (&key_bytes[..]).try_into().unwrap();
    let mut account = LocalAccount::new(address, ed25519_private_key, 0);

    let txn0 = context.create_user_account(&account).await;
    context.commit_block(&vec![txn0]).await;
    let txn1 = context.mint_user_account(&account).await;
    context.commit_block(&vec![txn1]).await;
    let txn2 = context.create_user_account(&other).await;
    context.commit_block(&vec![txn2]).await;

    let ed22519_txn = context.account_transfer(&mut account, &other, 5);
    let secp256k1_ecdsa_txn = ed22519_txn
        .into_raw_transaction()
        .sign_secp256k1_ecdsa(&private_key, public_key)
        .unwrap();
    let balance_start = context.get_apt_balance(other.address()).await;
    let bcs_txn = bcs::to_bytes(&secp256k1_ecdsa_txn.into_inner()).unwrap();
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", bcs_txn)
        .await;
    context.commit_mempool_txns(1).await;
    assert_eq!(
        balance_start + 5,
        context.get_apt_balance(other.address()).await
    );
}
