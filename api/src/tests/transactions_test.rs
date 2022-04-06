// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    current_function_name,
    tests::{assert_json, new_test_context, pretty, TestContext},
};

use aptos_api_types::HexEncodedBytes;
use aptos_crypto::{
    multi_ed25519::{MultiEd25519PrivateKey, MultiEd25519PublicKey},
    PrivateKey, SigningKey, Uniform,
};
use aptos_sdk::types::LocalAccount;
use aptos_types::{
    access_path::{AccessPath, Path},
    account_address::AccountAddress,
    transaction::{
        authenticator::{AuthenticationKey, TransactionAuthenticator},
        ChangeSet, Script, ScriptFunction, SignedTransaction,
    },
    write_set::{WriteOp, WriteSetMut},
};

use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_types::state_store::state_key::StateKey;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;

#[tokio::test]
async fn test_deserialize_genesis_transaction() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/transactions/0").await;
    serde_json::from_value::<aptos_api_types::Transaction>(resp).unwrap();
}

#[tokio::test]
async fn test_get_transactions_output_genesis_transaction() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/transactions").await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_returns_last_page_when_start_version_is_not_specified() {
    let mut context = new_test_context(current_function_name!());

    let mut root_account = context.root_account();
    for _i in 0..20 {
        let account = context.gen_account();
        let txn = context.create_user_account_by(&mut root_account, &account);
        context.commit_block(&vec![txn.clone()]).await;
    }

    let resp = context.get("/transactions").await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_with_start_version_is_too_large() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/transactions?start=1000000&limit=10")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_with_invalid_start_version_param() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?start=hello")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_with_invalid_limit_param() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?limit=hello")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_with_zero_limit() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?limit=0")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_param_limit_exceeds_limit() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?limit=2000")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transactions_output_user_transaction_with_script_function_payload() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=1").await;
    assert_eq!(2, txns.as_array().unwrap().len());
    context.check_golden_output(txns);
}

// TODO: figure out correct module payload
#[ignore]
#[tokio::test]
async fn test_get_transactions_output_user_transaction_with_module_payload() {
    let mut context = new_test_context(current_function_name!());
    let code = "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200";
    let mut root_account = context.root_account();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .module(hex::decode(code).unwrap()),
    );
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let expected_txns = context.get_transactions(2, 1);
    assert_eq!(1, expected_txns.len());

    assert_json(
        txns[0]["payload"].clone(),
        json!({
            "type": "module_bundle_payload",
            "modules": [
                {
                    "bytecode": format!("0x{}", code),
                    "abi": {
                        "address": "0xb1e55ed",
                        "name": "MyModule",
                        "friends": [],
                        "exposed_functions": [
                            {
                                "name": "id",
                                "visibility": "public",
                                "generic_type_params": [],
                                "params": [],
                                "return": ["u8"]
                            }
                        ],
                        "structs": []
                    }
                },
            ]
        }),
    )
}

// writeset not supported
#[ignore]
#[tokio::test]
async fn test_get_transactions_output_user_transaction_with_write_set_payload() {
    let mut context = new_test_context(current_function_name!());
    let mut root_account = context.root_account();
    let code_address = AccountAddress::from_hex_literal("0x1").unwrap();
    let txn = root_account.sign_with_transaction_builder(
        context.transaction_factory().change_set(ChangeSet::new(
            WriteSetMut::new(vec![
                (
                    StateKey::AccessPath(AccessPath::new(
                        code_address,
                        bcs::to_bytes(&Path::Code(ModuleId::new(
                            code_address,
                            Identifier::new("AptosAccount").unwrap(),
                        )))
                        .unwrap(),
                    )),
                    WriteOp::Deletion,
                ),
                (
                    StateKey::AccessPath(AccessPath::new(
                        context.root_account().address(),
                        bcs::to_bytes(&Path::Resource(StructTag {
                            address: code_address,
                            module: Identifier::new("TestCoin").unwrap(),
                            name: Identifier::new("Balance").unwrap(),
                            type_params: vec![],
                        }))
                        .unwrap(),
                    )),
                    WriteOp::Deletion,
                ),
            ])
            .freeze()
            .unwrap(),
            vec![],
        )),
    );
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    context.check_golden_output(txns);
}

#[tokio::test]
async fn test_post_bcs_format_transaction() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;
    context.check_golden_output(resp.clone());

    // ensure ed25519 sig txn can be submitted into mempool by JSON format
    context
        .expect_status_code(202)
        .post("/transactions", resp)
        .await;
}

#[tokio::test]
async fn test_post_invalid_bcs_format_transaction() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", bcs::to_bytes("invalid data").unwrap())
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_post_invalid_signature_transaction() {
    let mut context = new_test_context(current_function_name!());
    let txn = context.create_invalid_signature_transaction();
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", &body)
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_post_transaction_rejected_by_mempool() {
    let mut context = new_test_context(current_function_name!());
    let account1 = context.gen_account();
    let account2 = context.gen_account();
    let txn1 = context.create_user_account(&account1);
    let txn2 = context.create_user_account(&account2);

    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", &bcs::to_bytes(&txn1).unwrap())
        .await;

    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", &bcs::to_bytes(&txn2).unwrap())
        .await;
    context.check_golden_output(resp);
}

#[ignore]
#[tokio::test]
async fn test_multi_agent_signed_transaction() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let factory = context.transaction_factory();
    let mut root_account = context.root_account();
    let secondary = context.root_account();
    let txn = root_account.sign_multi_agent_with_transaction_builder(
        vec![&secondary],
        factory.create_user_account(account.public_key()),
    );

    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let (sender, secondary_signers) = match txn.authenticator() {
        TransactionAuthenticator::MultiAgent {
            sender,
            secondary_signer_addresses: _,
            secondary_signers,
        } => (sender, secondary_signers),
        _ => panic!(
            "expecting TransactionAuthenticator::MultiAgent, but got: {:?}",
            txn.authenticator()
        ),
    };
    assert_json(
        resp["signature"].clone(),
        json!({
            "type": "multi_agent_signature",
            "sender": {
                "type": "ed25519_signature",
                "public_key": format!("0x{}", hex::encode(sender.public_key_bytes())),
                "signature": format!("0x{}", hex::encode(sender.signature_bytes())),
            },
            "secondary_signer_addresses": [
                secondary.address().to_hex_literal(),
            ],
            "secondary_signers": [
                {
                    "type": "ed25519_signature",
                    "public_key": format!("0x{}",hex::encode(secondary_signers[0].public_key_bytes())),
                    "signature": format!("0x{}", hex::encode(secondary_signers[0].signature_bytes())),
                }
            ]
        }),
    );

    // ensure multi agent txns can be submitted into mempool by JSON format
    context
        .expect_status_code(202)
        .post("/transactions", resp)
        .await;
}

#[ignore]
#[tokio::test]
async fn test_multi_ed25519_signed_transaction() {
    let mut context = new_test_context(current_function_name!());

    let private_key = MultiEd25519PrivateKey::generate_for_testing();
    let public_key = MultiEd25519PublicKey::from(&private_key);
    let auth_key = AuthenticationKey::multi_ed25519(&public_key);

    let factory = context.transaction_factory();
    let mut root_account = context.root_account();
    // TODO: migrate once multi-ed25519 is supported
    let create_account_txn = root_account.sign_with_transaction_builder(
        factory.create_user_account(&Ed25519PrivateKey::generate_for_testing().public_key()),
    );
    context.commit_block(&vec![create_account_txn]).await;

    let raw_txn = factory
        .mint(auth_key.derived_address(), 1000)
        .sender(auth_key.derived_address())
        .sequence_number(0)
        .expiration_timestamp_secs(u64::MAX) // set timestamp to max to ensure static raw transaction
        .build();

    let signature = private_key.sign(&raw_txn);
    let txn = SignedTransaction::new_multisig(raw_txn, public_key, signature.clone());

    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    assert_json(
        resp["signature"].clone(),
        json!({
          "type": "multi_ed25519_signature",
          "public_keys": [
            "0x9e4208caddd825f71957c9b12dbfbd13a23fb0ea23eb398fd7e1f418b51f8fbc",
            "0x4708a77bb9285ce3745ffdd48c51980326b625488209803228ff623f3768c64e",
            "0x852b13cd7a89b0c223d74504705e84c745d32261244ed233ef0285637a1dece0",
            "0x77e7fe2a510e4f14e15071fc420469ee287b64f2c8f8c0221b946a3fd9cbfef3",
            "0xd0c66cfef88b999f027347726bd54eda4675ae312af9146bfdc9e9fa702cc90a",
            "0xd316059933e0dd6415f00ce350962c8e94b46373b7fb5fb49687f3d6b9e3cb30",
            "0xf20e973e6dfeda74ca8e15f1a7aed9c87d67bd12e071fd3de4240368422712c9",
            "0xead82d6e9e3f3baeaa557bd7a431a1c6fe9f35a82c10fed123f362615ee7c2cd",
            "0x5c048c8c456ff9dd2810343bbd630fb45bf064317efae22c65a1535cf392c5d5",
            "0x861546d0818178f2b5f37af0fa712fe8ce3cceeda894b553ee274f3fbcb4b32f",
            "0xfe047a766a47719591348a4601afb3f38b0c77fa3f820e0298c064e7cde6763f"
          ],
          "signatures": [
                "0xcf9e7a0284434c568cefecd995d2f1c950b041513e815f9bdd8a42cb641c9b6dfcc692b767ace76f4171ef4fa032d3b4687e9944ffbb6b2ebe7033758e55a002",
                "0x840caf50f80da4ca2d4146458da3d93a0fd8e46796d231e36fa426614a10e372a25c2a4843367f6a632fa2459fd6bd8f0a4b35febad4fbdb780fcfba36d81f0b",
                "0xe1523537cc3d2be86df0c65a03cc1168c4d10e9436d8f69bce0e229f8e91c1714a0440e57d9813eedb495a39790fb9090b688173634bfbefe55e194384c45b05"

          ],
          "threshold": 3,
          "bitmap": "0xe0000000"
        }),
    );

    // ensure multi sig txns can be submitted into mempool by JSON format
    context
        .expect_status_code(202)
        .post("/transactions", resp)
        .await;
}

#[tokio::test]
async fn test_get_transaction_by_hash() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let resp = context
        .get(&format!(
            "/transactions/{}",
            txns[0]["hash"].as_str().unwrap()
        ))
        .await;
    assert_json(resp, txns[0].clone());
}

#[tokio::test]
async fn test_get_transaction_by_hash_not_found() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(404)
        .get("/transactions/0xdadfeddcca7cb6396c735e9094c76c6e4e9cb3e3ef814730693aed59bd87b31d")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transaction_by_invalid_hash() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(400)
        .get("/transactions/0x1")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transaction_by_version_not_found() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(404)
        .get("/transactions/10000")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_get_transaction_by_version() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let resp = context.get("/transactions/2").await;
    assert_json(resp, txns[0].clone())
}

#[tokio::test]
async fn test_get_pending_transaction_by_hash() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    let body = bcs::to_bytes(&txn).unwrap();
    let pending_txn = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let txn_hash = pending_txn["hash"].as_str().unwrap();

    let txn = context.get(&format!("/transactions/{}", txn_hash)).await;
    assert_json(txn, pending_txn);

    let not_found = context
        .expect_status_code(404)
        .get("/transactions/0xdadfeddcca7cb6396c735e9094c76c6e4e9cb3e3ef814730693aed59bd87b31d")
        .await;
    context.check_golden_output(not_found);
}

#[tokio::test]
async fn test_signing_message_with_script_function_payload() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);

    let payload = json!({
        "type": "script_function_payload",
        "function": "0x1::AptosAccount::create_account",
        "type_arguments": [
        ],
        "arguments": [
            account.address().to_hex_literal(), // new_account_address
        ]
    });
    test_signing_message_with_payload(context, txn, payload).await;
}

// need a correct module payload
#[ignore]
#[tokio::test]
async fn test_signing_message_with_module_payload() {
    let context = new_test_context(current_function_name!());
    let code = "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200";
    let mut root_account = context.root_account();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .module(hex::decode(code).unwrap()),
    );
    let payload = json!({
            "type": "module_bundle_payload",
            "modules" : [
                {"bytecode": format!("0x{}", code)},
            ],
    });

    test_signing_message_with_payload(context, txn, payload).await;
}

#[tokio::test]
async fn test_signing_message_with_write_set_payload() {
    // This test is created for testing error message for now.
    // Update test when write_set_payload is supported
    let context = new_test_context(current_function_name!());
    let mut root_account = context.root_account();
    let code_address = AccountAddress::from_hex_literal("0x1").unwrap();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .change_set(ChangeSet::new(
                WriteSetMut::new(vec![
                    (
                        StateKey::AccessPath(AccessPath::new(
                            code_address,
                            bcs::to_bytes(&Path::Code(ModuleId::new(
                                code_address,
                                Identifier::new("AptosAccount").unwrap(),
                            )))
                            .unwrap(),
                        )),
                        WriteOp::Deletion,
                    ),
                    (
                        StateKey::AccessPath(AccessPath::new(
                            context.root_account().address(),
                            bcs::to_bytes(&Path::Resource(StructTag {
                                address: code_address,
                                module: Identifier::new("TestCoin").unwrap(),
                                name: Identifier::new("Balance").unwrap(),
                                type_params: vec![],
                            }))
                            .unwrap(),
                        )),
                        WriteOp::Deletion,
                    ),
                ])
                .freeze()
                .unwrap(),
                vec![],
            ))
            .expiration_timestamp_secs(u64::MAX),
    );
    let payload = json!({
        "type": "write_set_payload",
        "write_set": {
            "type": "direct_write_set",
            "changes": [
                {
                    "type": "delete_module",
                    "address": "0x1",
                    "module": "0x1::AptosAccount"
                },
                {
                    "type": "delete_resource",
                    "address": "0xb1e55ed",
                    "resource": "0x1::TestCoin::Balance"
                }
            ],
            "events": []
        }
    });

    let sender = context.root_account();
    let body = json!({
        "sender": sender.address().to_hex_literal(),
        "sequence_number": sender.sequence_number().to_string(),
        "gas_unit_price": txn.gas_unit_price().to_string(),
        "max_gas_amount": txn.max_gas_amount().to_string(),
        "gas_currency_code": txn.gas_currency_code(),
        "expiration_timestamp_secs": txn.expiration_timestamp_secs().to_string(),
        "payload": payload,
    });

    context
        .expect_status_code(400)
        .post("/transactions/signing_message", body)
        .await;
}

async fn test_signing_message_with_payload(
    mut context: TestContext,
    txn: SignedTransaction,
    payload: serde_json::Value,
) {
    let sender = context.root_account();
    let mut body = json!({
        "sender": sender.address().to_hex_literal(),
        "sequence_number": sender.sequence_number().to_string(),
        "gas_unit_price": txn.gas_unit_price().to_string(),
        "max_gas_amount": txn.max_gas_amount().to_string(),
        "gas_currency_code": txn.gas_currency_code(),
        "expiration_timestamp_secs": txn.expiration_timestamp_secs().to_string(),
        "payload": payload,
    });

    let resp = context
        .post("/transactions/signing_message", body.clone())
        .await;

    let signing_msg = resp["message"].as_str().unwrap();
    assert_eq!(
        signing_msg,
        format!(
            "0x{}",
            hex::encode(&txn.clone().into_raw_transaction().signing_message())
        )
    );

    let hex_bytes: HexEncodedBytes = signing_msg.parse().unwrap();
    let sig = context
        .root_account()
        .private_key()
        .sign_arbitrary_message(hex_bytes.inner());
    let expected_sig = match txn.authenticator() {
        TransactionAuthenticator::Ed25519 {
            public_key: _,
            signature,
        } => signature,
        _ => panic!("expect TransactionAuthenticator::Ed25519"),
    };
    assert_eq!(sig, expected_sig);

    // assert transaction can be submitted into mempool and execute.
    body["signature"] = json!({
        "type": "ed25519_signature",
        "public_key": format!("0x{}", hex::encode(sender.public_key().to_bytes())),
        "signature": format!("0x{}", hex::encode(sig.to_bytes())),
    });

    context
        .expect_status_code(202)
        .post("/transactions", body)
        .await;

    context.commit_mempool_txns(10).await;

    let ledger = context.get("/").await;
    assert_eq!(ledger["ledger_version"].as_str().unwrap(), "2"); // one metadata + one txn
}

#[tokio::test]
async fn test_get_account_transactions() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn]).await;

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions",
                context.root_account().address()
            )
            .as_str(),
        )
        .await;
    assert_eq!(1, txns.as_array().unwrap().len());
    let expected_txns = context.get("/transactions?start=2&limit=1").await;
    assert_json(txns, expected_txns);
}

#[tokio::test]
async fn test_get_account_transactions_filter_transactions_by_start_sequence_number() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn]).await;

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start=1",
                context.root_account().address()
            )
            .as_str(),
        )
        .await;
    assert_json(txns, json!([]));
}

#[tokio::test]
async fn test_get_account_transactions_filter_transactions_by_start_sequence_number_is_too_large() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn]).await;

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start=1000",
                context.root_account().address()
            )
            .as_str(),
        )
        .await;
    assert_json(txns, json!([]));
}

#[tokio::test]
async fn test_get_account_transactions_filter_transactions_by_limit() {
    let mut context = new_test_context(current_function_name!());
    let mut root_account = context.root_account();
    let account1 = context.gen_account();
    let txn1 = context.create_user_account_by(&mut root_account, &account1);
    let account2 = context.gen_account();
    let txn2 = context.create_user_account_by(&mut root_account, &account2);
    context.commit_block(&vec![txn1, txn2]).await;

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start=0&limit=1",
                context.root_account().address()
            )
            .as_str(),
        )
        .await;
    assert_eq!(txns.as_array().unwrap().len(), 1);

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start=0&limit=2",
                context.root_account().address()
            )
            .as_str(),
        )
        .await;
    assert_eq!(txns.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_module_payload_bytecode() {
    let context = new_test_context(current_function_name!());
    let invalid_bytecode = hex::decode("a11ceb0b030000").unwrap();
    let mut root_account = context.root_account();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .module(invalid_bytecode)
            .expiration_timestamp_secs(u64::MAX),
    );
    test_transaction_vm_status(context, txn, false).await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_script_payload_bytecode() {
    let context = new_test_context(current_function_name!());
    let mut root_account = context.root_account();
    let invalid_bytecode = hex::decode("a11ceb0b030000").unwrap();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .script(Script::new(invalid_bytecode, vec![], vec![]))
            .expiration_timestamp_secs(u64::MAX),
    );
    test_transaction_vm_status(context, txn, false).await
}

#[ignore]
#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_write_set_payload() {
    let context = new_test_context(current_function_name!());

    let invalid_bytecode = hex::decode("a11ceb0b030000").unwrap();
    let mut root_account = context.root_account();
    let code_address = AccountAddress::from_hex_literal("0x1").unwrap();
    let txn = root_account.sign_with_transaction_builder(
        context.transaction_factory().change_set(ChangeSet::new(
            WriteSetMut::new(vec![(
                StateKey::AccessPath(AccessPath::new(
                    code_address,
                    bcs::to_bytes(&Path::Code(ModuleId::new(
                        code_address,
                        Identifier::new("AptosAccount").unwrap(),
                    )))
                    .unwrap(),
                )),
                WriteOp::Value(invalid_bytecode),
            )])
            .freeze()
            .unwrap(),
            vec![],
        )),
    );

    // should fail, but VM executed successfully, need investigate, but out of API scope
    test_transaction_vm_status(context, txn, true).await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_script_function_address() {
    let context = new_test_context(current_function_name!());
    let account = context.root_account();
    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        account,
        "0x1222",
        "TestCoin",
        "transfer",
        vec![],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
    .await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_script_function_module_name() {
    let context = new_test_context(current_function_name!());
    let account = context.root_account();
    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        account,
        "0x1",
        "TestCoinInvalid",
        "transfer",
        vec![],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
    .await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_script_function_name() {
    let context = new_test_context(current_function_name!());
    let account = context.root_account();
    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        account,
        "0x1",
        "TestCoin",
        "transfer_invalid",
        vec![],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
    .await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_invalid_script_function_arguments() {
    let context = new_test_context(current_function_name!());
    let account = context.root_account();
    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        account,
        "0x1",
        "TestCoin",
        "transfer",
        vec![],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u8).unwrap(), // invalid type
        ],
    )
    .await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_missing_script_function_arguments() {
    let context = new_test_context(current_function_name!());
    let account = context.root_account();
    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        account,
        "0x1",
        "TestCoin",
        "transfer",
        vec![],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            // missing arguments
        ],
    )
    .await
}

#[tokio::test]
async fn test_get_txn_execute_failed_by_script_function_validation() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    context
        .commit_block(&vec![context.create_user_account(&account)])
        .await;

    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        account,
        "0x1",
        "TestCoin",
        "transfer",
        vec![],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&123u64).unwrap(), // exceed limit, account balance is 0.
        ],
    )
    .await
}

#[ignore] // re-enable after cleaning after compiled code
#[tokio::test]
async fn test_get_txn_execute_failed_by_script_function_execution_failure() {
    let mut context = new_test_context(current_function_name!());

    // address 0xA550C18 {
    //     module Hello {
    //         fun world() {
    //             1/0;
    //         }
    //         public(script) fun hello() {
    //             world();
    //         }
    //     }
    // }
    let hello_script_fun = hex::decode("a11ceb0b030000000601000203020a050c01070d12081f100c2f24000000010000000002000000000548656c6c6f0568656c6c6f05776f726c640000000000000000000000000a550c180002000000021101020100000000050601000000000000000600000000000000001a010200").unwrap();
    let mut root_account = context.root_account();
    let module_txn = root_account
        .sign_with_transaction_builder(context.transaction_factory().module(hello_script_fun));

    context.commit_block(&vec![module_txn]).await;

    test_get_txn_execute_failed_by_invalid_script_function(
        context,
        root_account,
        "0xA550C18",
        "Hello",
        "hello",
        vec![],
        vec![],
    )
    .await
}

#[ignore]
#[tokio::test]
async fn test_get_txn_execute_failed_by_script_execution_failure() {
    let context = new_test_context(current_function_name!());

    // script {
    //     fun main() {
    //         1/0;
    //     }
    // }
    let script =
        hex::decode("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102")
            .unwrap();
    let mut root_account = context.root_account();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .script(Script::new(script, vec![], vec![])),
    );

    test_transaction_vm_status(context, txn, false).await
}

async fn test_get_txn_execute_failed_by_invalid_script_function(
    context: TestContext,
    mut account: LocalAccount,
    address: &str,
    module_id: &str,
    func: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) {
    let txn = account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .script_function(ScriptFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal(address).unwrap(),
                    Identifier::new(module_id).unwrap(),
                ),
                Identifier::new(func).unwrap(),
                ty_args,
                args,
            ))
            .expiration_timestamp_secs(u64::MAX),
    );

    test_transaction_vm_status(context, txn, false).await
}

async fn test_transaction_vm_status(
    mut context: TestContext,
    txn: SignedTransaction,
    success: bool,
) {
    let body = bcs::to_bytes(&txn).unwrap();
    // we don't validate transaction payload when submit txn into mempool.
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;
    context.commit_mempool_txns(1).await;
    let resp = context
        .get(format!("/transactions/{}", txn.committed_hash().to_hex_literal()).as_str())
        .await;
    assert_eq!(
        resp["success"].as_bool().unwrap(),
        success,
        "{}",
        pretty(&resp)
    );
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_submit_transaction_rejects_payload_too_large_bcs_txn_body() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(413)
        .post_bcs_txn(
            "/transactions",
            gen_string(context.context.content_length_limit() + 1).as_bytes(),
        )
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_submit_transaction_rejects_payload_too_large_json_body() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(413)
        .post(
            "/transactions",
            json!({
                "data": gen_string(context.context.content_length_limit()+1).as_bytes(),
            }),
        )
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_submit_transaction_rejects_invalid_content_type() {
    let mut context = new_test_context(current_function_name!());
    let req = warp::test::request()
        .header("content-type", "invalid")
        .method("POST")
        .body("text")
        .path("/transactions");

    let resp = context.expect_status_code(415).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_submit_transaction_rejects_invalid_json() {
    let mut context = new_test_context(current_function_name!());
    let req = warp::test::request()
        .header("content-type", "application/json")
        .method("POST")
        .body("invalid json")
        .path("/transactions");

    let resp = context.expect_status_code(400).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_create_signing_message_rejects_payload_too_large_json_body() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(413)
        .post(
            "/transactions/signing_message",
            json!({
                "data": gen_string(context.context.content_length_limit()+1).as_bytes(),
            }),
        )
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_create_signing_message_rejects_invalid_content_type() {
    let mut context = new_test_context(current_function_name!());
    let req = warp::test::request()
        .header("content-type", "invalid")
        .method("POST")
        .body("text")
        .path("/transactions/signing_message");

    let resp = context.expect_status_code(415).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_create_signing_message_rejects_invalid_json() {
    let mut context = new_test_context(current_function_name!());
    let req = warp::test::request()
        .header("content-type", "application/json")
        .method("POST")
        .body("invalid json")
        .path("/transactions/signing_message");

    let resp = context.expect_status_code(400).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_create_signing_message_rejects_no_content_length_request() {
    let mut context = new_test_context(current_function_name!());
    let req = warp::test::request()
        .header("content-type", "application/json")
        .method("POST")
        .path("/transactions/signing_message");

    let resp = context.expect_status_code(411).execute(req).await;
    context.check_golden_output(resp);
}

fn gen_string(len: u64) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(len as usize)
        .map(char::from)
        .collect()
}
