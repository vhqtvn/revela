// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use anyhow::anyhow;
use aptos::common::types::{GasOptions, DEFAULT_GAS_UNIT_PRICE, DEFAULT_MAX_GAS};
use aptos::test::INVALID_ACCOUNT;
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::config::PersistableConfig;
use aptos_config::{config::ApiConfig, utils::get_available_port};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_crypto::{HashValue, PrivateKey};
use aptos_rest_client::aptos_api_types::UserTransaction;
use aptos_rest_client::Transaction;
use aptos_rosetta::common::BlockHash;
use aptos_rosetta::types::{
    AccountIdentifier, BlockResponse, Operation, OperationStatusType, OperationType,
    TransactionType,
};
use aptos_rosetta::{
    client::RosettaClient,
    common::{native_coin, BLOCKCHAIN, Y2K_MS},
    types::{
        AccountBalanceRequest, AccountBalanceResponse, BlockIdentifier, BlockRequest,
        NetworkIdentifier, NetworkRequest, PartialBlockIdentifier,
    },
    ROSETTA_VERSION,
};
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_types::transaction::SignedTransaction;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use cached_packages::aptos_stdlib;
use forge::{LocalSwarm, Node, NodeExt, Swarm};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{future::Future, time::Duration};
use tokio::{task::JoinHandle, time::Instant};

const DEFAULT_MAX_WAIT_MS: u64 = 5000;
const DEFAULT_INTERVAL_MS: u64 = 100;
static DEFAULT_MAX_WAIT_DURATION: Duration = Duration::from_millis(DEFAULT_MAX_WAIT_MS);
static DEFAULT_INTERVAL_DURATION: Duration = Duration::from_millis(DEFAULT_INTERVAL_MS);

pub async fn setup_test(
    num_nodes: usize,
    num_accounts: usize,
) -> (LocalSwarm, CliTestFramework, JoinHandle<()>, RosettaClient) {
    let (swarm, cli, faucet) = SwarmBuilder::new_local(num_nodes)
        .with_aptos()
        .build_with_cli(num_accounts)
        .await;
    let validator = swarm.validators().next().unwrap();

    // And the client
    let rosetta_port = get_available_port();
    let rosetta_socket_addr = format!("127.0.0.1:{}", rosetta_port);
    let rosetta_url = format!("http://{}", rosetta_socket_addr.clone())
        .parse()
        .unwrap();
    let rosetta_client = RosettaClient::new(rosetta_url);
    let api_config = ApiConfig {
        enabled: true,
        address: rosetta_socket_addr.parse().unwrap(),
        tls_cert_path: None,
        tls_key_path: None,
        content_length_limit: None,
        ..Default::default()
    };

    // Start the server
    let _rosetta = aptos_rosetta::bootstrap_async(
        swarm.chain_id(),
        api_config,
        Some(aptos_rest_client::Client::new(
            validator.rest_api_endpoint(),
        )),
    )
    .await
    .unwrap();

    // Ensure rosetta can take requests
    try_until_ok_default(|| rosetta_client.network_list())
        .await
        .unwrap();

    (swarm, cli, faucet, rosetta_client)
}

#[tokio::test]
async fn test_network() {
    let (swarm, _, _, rosetta_client) = setup_test(1, 1).await;
    let chain_id = swarm.chain_id();

    // We only support one network, this network
    let networks = try_until_ok_default(|| rosetta_client.network_list())
        .await
        .unwrap();
    assert_eq!(1, networks.network_identifiers.len());
    let network_id = networks.network_identifiers.first().unwrap();
    assert_eq!(BLOCKCHAIN, network_id.blockchain);
    assert_eq!(chain_id.to_string(), network_id.network);

    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };
    let options = rosetta_client.network_options(&request).await.unwrap();
    assert_eq!(ROSETTA_VERSION, options.version.rosetta_version);

    // TODO: Check other options

    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };
    let status = try_until_ok_default(|| rosetta_client.network_status(&request))
        .await
        .unwrap();
    assert!(status.current_block_timestamp >= Y2K_MS);
    assert_eq!(
        BlockIdentifier {
            index: 0,
            hash: BlockHash::new(chain_id, 0).to_string()
        },
        status.genesis_block_identifier
    );
    assert_eq!(
        status.genesis_block_identifier,
        status.oldest_block_identifier,
    );
}

#[tokio::test]
async fn test_account_balance() {
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 2).await;

    let account_1 = cli.account_id(0);
    let account_2 = cli.account_id(1);
    let chain_id = swarm.chain_id();

    // At time 0, there should be 0 balance
    let response = get_balance(&rosetta_client, chain_id, account_1, Some(0))
        .await
        .unwrap();
    assert_eq!(
        response.block_identifier,
        BlockIdentifier {
            index: 0,
            hash: BlockHash::new(chain_id, 0).to_string()
        }
    );

    // First fund account 1 with lots more gas
    cli.fund_account(0, Some(DEFAULT_FUNDED_COINS * 2))
        .await
        .unwrap();

    let mut account_1_balance = DEFAULT_FUNDED_COINS * 3;
    let mut account_2_balance = DEFAULT_FUNDED_COINS;
    // At some time both accounts should exist with initial amounts
    try_until_ok(Duration::from_secs(5), DEFAULT_INTERVAL_DURATION, || {
        account_has_balance(&rosetta_client, chain_id, account_1, account_1_balance, 0)
    })
    .await
    .unwrap();
    try_until_ok_default(|| {
        account_has_balance(&rosetta_client, chain_id, account_2, account_2_balance, 0)
    })
    .await
    .unwrap();

    // Send money, and expect the gas and fees to show up accordingly
    const TRANSFER_AMOUNT: u64 = 5000;
    let response = cli
        .transfer_coins(
            0,
            1,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap();
    account_1_balance -= TRANSFER_AMOUNT + response.gas_used * response.gas_unit_price;
    account_2_balance += TRANSFER_AMOUNT;
    account_has_balance(&rosetta_client, chain_id, account_1, account_1_balance, 1)
        .await
        .unwrap();
    account_has_balance(&rosetta_client, chain_id, account_2, account_2_balance, 0)
        .await
        .unwrap();

    // Failed transaction spends gas
    let _ = cli
        .transfer_invalid_addr(
            0,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap_err();

    // Make a bad transaction, which will cause gas to be spent but no transfer
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let txns = rest_client
        .get_account_transactions(account_1, None, None)
        .await
        .unwrap()
        .into_inner();
    let failed_txn = txns.last().unwrap();
    if let Transaction::UserTransaction(txn) = failed_txn {
        account_1_balance -= txn.request.gas_unit_price.0 * txn.info.gas_used.0;
        account_has_balance(&rosetta_client, chain_id, account_1, account_1_balance, 2)
            .await
            .unwrap();
    }

    // Check that the balance hasn't changed (and should be 0) in the invalid account
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap(),
        0,
        0,
    )
    .await
    .unwrap();
}

async fn account_has_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account: AccountAddress,
    expected_balance: u64,
    expected_sequence_number: u64,
) -> anyhow::Result<u64> {
    let response = get_balance(rosetta_client, chain_id, account, None).await?;
    assert_eq!(
        expected_sequence_number,
        response.metadata.sequence_number.0
    );

    if response.balances.iter().any(|amount| {
        amount.currency == native_coin() && amount.value == expected_balance.to_string()
    }) {
        Ok(response.block_identifier.index)
    } else {
        Err(anyhow!(
            "Failed to find account with {} {:?}, received {:?}",
            expected_balance,
            native_coin(),
            response
        ))
    }
}

async fn get_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account: AccountAddress,
    index: Option<u64>,
) -> anyhow::Result<AccountBalanceResponse> {
    let request = AccountBalanceRequest {
        network_identifier: chain_id.into(),
        account_identifier: account.into(),
        block_identifier: Some(PartialBlockIdentifier { index, hash: None }),
        currencies: Some(vec![native_coin()]),
    };
    try_until_ok_default(|| rosetta_client.account_balance(&request)).await
}

#[tokio::test]
async fn test_transfer() {
    let (mut swarm, cli, _faucet, rosetta_client) = setup_test(1, 1).await;
    let chain_id = swarm.chain_id();
    let public_info = swarm.aptos_public_info();
    let client = public_info.client();
    let sender = cli.account_id(0);
    let receiver = AccountAddress::from_hex_literal("0xBEEF").unwrap();
    let sender_private_key = cli.private_key(0);
    let sender_balance = client
        .get_account_balance(sender)
        .await
        .unwrap()
        .into_inner()
        .coin
        .value
        .0;
    let network = NetworkIdentifier::from(chain_id);

    // Wait until the Rosetta service is ready
    let request = NetworkRequest {
        network_identifier: network.clone(),
    };

    loop {
        let status = try_until_ok_default(|| rosetta_client.network_status(&request))
            .await
            .unwrap();
        if status.current_block_identifier.index >= 2 {
            break;
        }
    }
    // Attempt to transfer all coins to another user (should fail)
    rosetta_client
        .transfer(
            &network,
            sender_private_key,
            receiver,
            sender_balance,
            expiry_time(Duration::from_secs(5)).as_secs(),
            None,
            None,
            None,
        )
        .await
        .expect_err("Should fail simulation since we can't transfer all coins");

    // Attempt to transfer more than balance to another user (should fail)
    rosetta_client
        .transfer(
            &network,
            sender_private_key,
            receiver,
            sender_balance + 200,
            expiry_time(Duration::from_secs(5)).as_secs(),
            None,
            None,
            None,
        )
        .await
        .expect_err("Should fail simulation since we can't transfer more than balance coins");

    // Attempt to transfer more than balance to another user (should fail)
    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(1)
        .with_max_gas_amount(500);
    let txn_payload = aptos_stdlib::aptos_account_transfer(receiver, 100);
    let unsigned_transaction = transaction_factory
        .payload(txn_payload)
        .sender(sender)
        .sequence_number(0)
        .build();
    let signed_transaction = SignedTransaction::new(
        unsigned_transaction,
        sender_private_key.public_key(),
        Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
    );

    let simulation_txn = client
        .simulate_bcs(&signed_transaction)
        .await
        .expect("Should succeed getting gas estimate")
        .into_inner();
    let gas_usage = simulation_txn.info.gas_used();

    // Attempt to transfer more than balance - gas to another user (should fail)
    rosetta_client
        .transfer(
            &network,
            sender_private_key,
            receiver,
            sender_balance - gas_usage + 1,
            expiry_time(Duration::from_secs(5)).as_secs(),
            None,
            None,
            None,
        )
        .await
        .expect_err("Should fail simulation since we can't transfer more than balance + gas coins");

    // Attempt to transfer more than balance - gas to another user (should fail)
    let transfer = transfer_and_wait(
        &rosetta_client,
        client,
        &network,
        sender_private_key,
        receiver,
        sender_balance - gas_usage,
        Duration::from_secs(5),
        None,
        None,
        None,
    )
    .await
    .expect("Should succeed transfer");
    assert_eq!(transfer.info.gas_used.0, gas_usage);

    // Sender balance should be 0
    assert_eq!(
        client
            .get_account_balance(sender)
            .await
            .unwrap()
            .into_inner()
            .coin
            .value
            .0,
        0
    );
    // Receiver should be sent coins
    assert_eq!(
        client
            .get_account_balance(receiver)
            .await
            .unwrap()
            .into_inner()
            .coin
            .value
            .0,
        sender_balance - gas_usage
    );
}

/// This test tests all of Rosetta's functionality from the read side in one go.  Since
/// it's block based and it needs time to run, we do all the checks in a single test.
#[tokio::test]
async fn test_block() {
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 5).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();

    // Mapping of account to block and balance mappings
    let mut balances = BTreeMap::<AccountAddress, BTreeMap<u64, i128>>::new();

    // Wait until the Rosetta service is ready
    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };

    loop {
        let status = try_until_ok_default(|| rosetta_client.network_status(&request))
            .await
            .unwrap();
        if status.current_block_identifier.index >= 2 {
            break;
        }
    }

    // Do some transfers
    let account_id_0 = cli.account_id(0);
    let account_id_1 = cli.account_id(1);
    let account_id_3 = cli.account_id(3);

    cli.fund_account(0, Some(10000000)).await.unwrap();
    cli.fund_account(1, Some(650000)).await.unwrap();
    cli.fund_account(2, Some(50000)).await.unwrap();
    cli.fund_account(3, Some(20000)).await.unwrap();

    let private_key_0 = cli.private_key(0);
    let private_key_1 = cli.private_key(1);
    let private_key_2 = cli.private_key(2);
    let private_key_3 = cli.private_key(3);
    let network_identifier = chain_id.into();
    let seq_no_0 = transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_0,
        account_id_1,
        20,
        Duration::from_secs(5),
        Some(0),
        None,
        None,
    )
    .await
    .unwrap()
    .request
    .sequence_number
    .0;
    transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_1,
        account_id_0,
        20,
        Duration::from_secs(5),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_0,
        account_id_0,
        20,
        Duration::from_secs(5),
        Some(seq_no_0 + 1),
        None,
        None,
    )
    .await
    .unwrap();
    // Create a new account via transfer
    transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_2,
        AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap(),
        20,
        Duration::from_secs(5),
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let seq_no_3 = transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_3,
        account_id_0,
        20,
        Duration::from_secs(5),
        None,
        Some(20000),
        Some(1),
    )
    .await
    .unwrap()
    .request
    .sequence_number
    .0;

    // Create another account via command
    create_account_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_3,
        AccountAddress::from_hex_literal("0x99").unwrap(),
        Duration::from_secs(5),
        Some(seq_no_3 + 1),
        None,
        None,
    )
    .await
    .unwrap();

    transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_1,
        account_id_3,
        20,
        Duration::from_secs(5),
        // Test the default behavior
        None,
        None,
        Some(2),
    )
    .await
    .unwrap();

    // This one will fail because expiration is in the past
    transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_3,
        AccountAddress::ONE,
        20,
        Duration::from_secs(0),
        None,
        None,
        None,
    )
    .await
    .unwrap_err();

    // This one will fail because gas is too low
    transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_3,
        AccountAddress::ONE,
        20,
        Duration::from_secs(5),
        None,
        Some(1),
        None,
    )
    .await
    .unwrap_err();

    // This one will fail
    let maybe_final_txn = transfer_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        private_key_1,
        AccountAddress::ONE,
        20,
        Duration::from_secs(5),
        None,
        Some(100000),
        None,
    )
    .await
    .unwrap_err();

    let final_txn = match maybe_final_txn {
        ErrorWrapper::BeforeSubmission(err) => {
            panic!("Failed prior to submission of transaction {:?}", err)
        }
        ErrorWrapper::AfterSubmission(txn) => txn,
    };

    let final_block_to_check = rest_client
        .get_block_by_version(final_txn.info.version.0, false)
        .await
        .expect("Should be able to get block info for completed txns");

    // Check a couple blocks past the final transaction to check more txns
    let final_block_height = final_block_to_check.into_inner().block_height.0 + 2;

    // TODO: Track total supply?
    // TODO: Check no repeated block hashes
    // TODO: Check no repeated txn hashes (in a block)
    // TODO: Check account balance block hashes?
    // TODO: Handle multiple coin types

    eprintln!("Checking blocks 0..{}", final_block_height);

    // Wait until the Rosetta service is ready
    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };

    loop {
        let status = try_until_ok_default(|| rosetta_client.network_status(&request))
            .await
            .unwrap();
        if status.current_block_identifier.index >= final_block_height {
            break;
        }
    }

    // Now we have to watch all the changes
    let mut current_version = 0;
    let mut previous_block_index = 0;
    for block_height in 0..final_block_height {
        let request = BlockRequest::by_index(chain_id, block_height);
        let response: BlockResponse = rosetta_client
            .block(&request)
            .await
            .expect("Should be able to get blocks that are already known");
        let block = response.block;
        let actual_block = rest_client
            .get_block_by_height(block_height, true)
            .await
            .expect("Should be able to get block for a known block")
            .into_inner();

        assert_eq!(
            block.block_identifier.index, block_height,
            "The block should match the requested block"
        );
        assert_eq!(
            block.block_identifier.hash,
            BlockHash::new(chain_id, block_height).to_string(),
            "Block hash should match chain_id-block_height"
        );
        assert_eq!(
            block.parent_block_identifier.index, previous_block_index,
            "Parent block index should be previous block"
        );
        assert_eq!(
            block.parent_block_identifier.hash,
            BlockHash::new(chain_id, previous_block_index).to_string(),
            "Parent block hash should be previous block chain_id-block_height"
        );

        // It's only greater or equal because microseconds are cut off
        let expected_timestamp = if block_height == 0 {
            Y2K_MS
        } else {
            actual_block.block_timestamp.0.saturating_div(1000)
        };
        assert_eq!(
            expected_timestamp, block.timestamp,
            "Block timestamp should match actual timestamp but in ms"
        );

        // First transaction should be first in block
        assert_eq!(
            current_version, actual_block.first_version.0,
            "First transaction in block should be the current version"
        );

        let actual_txns = actual_block
            .transactions
            .as_ref()
            .expect("Every actual block should have transactions");
        parse_block_transactions(&block, &mut balances, actual_txns, &mut current_version).await;

        // The full block must have been processed
        assert_eq!(current_version - 1, actual_block.last_version.0);

        // Keep track of the previous
        previous_block_index = block_height;
    }

    // Reconcile and ensure all balances are calculated correctly
    check_balances(&rosetta_client, chain_id, balances).await;
}

/// Parse the transactions in each block
async fn parse_block_transactions(
    block: &aptos_rosetta::types::Block,
    balances: &mut BTreeMap<AccountAddress, BTreeMap<u64, i128>>,
    actual_txns: &[Transaction],
    current_version: &mut u64,
) {
    for (txn_number, transaction) in block.transactions.iter().enumerate() {
        let actual_txn = actual_txns
            .get(txn_number)
            .expect("There should be the same number of transactions in the actual block");
        let actual_txn_info = actual_txn
            .transaction_info()
            .expect("Actual transaction should not be pending and have transaction info");
        let txn_metadata = transaction.metadata;

        // Ensure transaction identifier is correct
        assert_eq!(
            *current_version, txn_metadata.version.0,
            "There should be no gaps in transaction versions"
        );
        assert_eq!(
            format!("{:x}", actual_txn_info.hash.0),
            transaction.transaction_identifier.hash,
            "Transaction hash should match the actual hash"
        );

        // Type specific checks
        match txn_metadata.transaction_type {
            TransactionType::Genesis => {
                assert_eq!(0, *current_version);
            }
            TransactionType::User => {}
            TransactionType::BlockMetadata | TransactionType::StateCheckpoint => {
                assert!(transaction.operations.is_empty());
            }
        }

        parse_operations(
            block.block_identifier.index,
            balances,
            transaction,
            actual_txn,
        )
        .await;

        for (_, account_balance) in balances.iter() {
            if let Some(amount) = account_balance.get(current_version) {
                assert!(*amount >= 0, "Amount shouldn't be negative!")
            }
        }

        // Increment to next version
        *current_version += 1;
    }
}

/// Parse the individual operations in a transaction
async fn parse_operations(
    block_height: u64,
    balances: &mut BTreeMap<AccountAddress, BTreeMap<u64, i128>>,
    transaction: &aptos_rosetta::types::Transaction,
    actual_txn: &Transaction,
) {
    // If there are no operations, then there is no gas operation
    let mut has_gas_op = false;
    for (expected_index, operation) in transaction.operations.iter().enumerate() {
        assert_eq!(expected_index as u64, operation.operation_identifier.index);

        // Gas transaction is always last
        let status = OperationStatusType::from_str(
            operation
                .status
                .as_ref()
                .expect("Should have an operation status"),
        )
        .expect("Operation status should be known");
        let operation_type = OperationType::from_str(&operation.operation_type)
            .expect("Operation type should be known");

        // Iterate through every operation, keeping track of balances
        match operation_type {
            OperationType::CreateAccount => {
                // Initialize state for a new account
                let account = operation
                    .account
                    .as_ref()
                    .expect("There should be an account in a create account operation")
                    .account_address()
                    .expect("Account address should be parsable");

                if actual_txn.success() {
                    assert_eq!(OperationStatusType::Success, status);
                    let account_balances = balances.entry(account).or_default();

                    if account_balances.is_empty() {
                        account_balances.insert(block_height, 0i128);
                    } else {
                        panic!("Account already has a balance when being created!");
                    }
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed create account operation"
                    );
                }
            }
            OperationType::Deposit => {
                let account = operation
                    .account
                    .as_ref()
                    .expect("There should be an account in a deposit operation")
                    .account_address()
                    .expect("Account address should be parsable");

                if actual_txn.success() {
                    assert_eq!(OperationStatusType::Success, status);
                    let account_balances = balances.entry(account).or_insert_with(|| {
                        let mut map = BTreeMap::new();
                        map.insert(block_height, 0);
                        map
                    });
                    let (_, latest_balance) = account_balances.iter().last().unwrap();
                    let amount = operation
                        .amount
                        .as_ref()
                        .expect("Should have an amount in a deposit operation");
                    assert_eq!(
                        amount.currency,
                        native_coin(),
                        "Balance should be the native coin"
                    );
                    let delta =
                        u64::parse(&amount.value).expect("Should be able to parse amount value");

                    // Add with panic on overflow in case of too high of a balance
                    let new_balance = *latest_balance + delta as i128;
                    account_balances.insert(block_height, new_balance);
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed deposit operation"
                    );
                }
            }
            OperationType::Withdraw => {
                // Gas is always successful
                if actual_txn.success() {
                    assert_eq!(OperationStatusType::Success, status);
                    let account = operation
                        .account
                        .as_ref()
                        .expect("There should be an account in a withdraw operation")
                        .account_address()
                        .expect("Account address should be parsable");

                    let account_balances = balances.entry(account).or_insert_with(|| {
                        let mut map = BTreeMap::new();
                        map.insert(block_height, 0);
                        map
                    });
                    let (_, latest_balance) = account_balances.iter().last().unwrap();
                    let amount = operation
                        .amount
                        .as_ref()
                        .expect("Should have an amount in a deposit operation");
                    assert_eq!(
                        amount.currency,
                        native_coin(),
                        "Balance should be the native coin"
                    );
                    let delta = u64::parse(
                        amount
                            .value
                            .strip_prefix('-')
                            .expect("Should have a negative number"),
                    )
                    .expect("Should be able to parse amount value");

                    // Subtract with panic on overflow in case of a negative balance
                    let new_balance = *latest_balance - delta as i128;
                    account_balances.insert(block_height, new_balance);
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed withdraw operation"
                    );
                }
            }
            OperationType::SetOperator => {
                if actual_txn.success() {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful set operator operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed set operator operation"
                    );
                }
            }
            OperationType::Fee => {
                has_gas_op = true;
                assert_eq!(OperationStatusType::Success, status);
                let account = operation
                    .account
                    .as_ref()
                    .expect("There should be an account in a fee operation")
                    .account_address()
                    .expect("Account address should be parsable");

                let account_balances = balances.entry(account).or_insert_with(|| {
                    let mut map = BTreeMap::new();
                    map.insert(block_height, 0);
                    map
                });
                let (_, latest_balance) = account_balances.iter().last().unwrap();
                let amount = operation
                    .amount
                    .as_ref()
                    .expect("Should have an amount in a fee operation");
                assert_eq!(
                    amount.currency,
                    native_coin(),
                    "Balance should be the native coin"
                );
                let delta = u64::parse(
                    amount
                        .value
                        .strip_prefix('-')
                        .expect("Should have a negative number"),
                )
                .expect("Should be able to parse amount value");

                // Subtract with panic on overflow in case of a negative balance
                let new_balance = *latest_balance - delta as i128;
                account_balances.insert(block_height, new_balance);

                match actual_txn {
                    Transaction::UserTransaction(txn) => {
                        assert_eq!(
                            txn.info
                                .gas_used
                                .0
                                .saturating_mul(txn.request.gas_unit_price.0),
                            delta,
                            "Gas operation should always match gas used * gas unit price"
                        )
                    }
                    _ => {
                        panic!("Gas transactions should be user transactions!")
                    }
                };
            }
        }
    }

    assert!(
        has_gas_op
            || transaction.metadata.transaction_type == TransactionType::Genesis
            || transaction.operations.is_empty(),
        "Must have a gas operation at least in a transaction except for Genesis",
    );
}

/// Check that all balances are correct with the account balance command from the blocks
async fn check_balances(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    balances: BTreeMap<AccountAddress, BTreeMap<u64, i128>>,
) {
    // TODO: Check some random times that arent on changes?
    for (account, account_balances) in balances {
        for (block_height, expected_balance) in account_balances {
            // Block should match it's calculated balance
            let response = rosetta_client
                .account_balance(&AccountBalanceRequest {
                    network_identifier: NetworkIdentifier::from(chain_id),
                    account_identifier: account.into(),
                    block_identifier: Some(PartialBlockIdentifier {
                        index: Some(block_height),
                        hash: None,
                    }),
                    currencies: Some(vec![native_coin()]),
                })
                .await
                .unwrap();
            assert_eq!(
                block_height, response.block_identifier.index,
                "Block should be the one expected"
            );

            let balance = response.balances.first().unwrap();
            assert_eq!(
                balance.currency,
                native_coin(),
                "Balance should be the native coin"
            );
            assert_eq!(
                expected_balance,
                u64::parse(&balance.value).expect("Should have a balance from account balance")
                    as i128
            );
        }
    }
}

#[tokio::test]
async fn test_invalid_transaction_gas_charged() {
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 1).await;
    let chain_id = swarm.chain_id();

    // Make sure first that there's money to transfer
    cli.assert_account_balance_now(0, DEFAULT_FUNDED_COINS)
        .await;

    // Now let's see some transfers
    const TRANSFER_AMOUNT: u64 = 5000;
    let _ = cli
        .transfer_invalid_addr(
            0,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap_err();

    let sender = cli.account_id(0);

    // Find failed transaction
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let txns = rest_client
        .get_account_transactions(sender, None, None)
        .await
        .unwrap()
        .into_inner();
    let actual_txn = txns.iter().find(|txn| !txn.success()).unwrap();
    let actual_txn = if let Transaction::UserTransaction(txn) = actual_txn {
        txn
    } else {
        panic!("Not a user transaction");
    };
    let txn_version = actual_txn.info.version.0;

    let block_info = rest_client
        .get_block_by_version(txn_version, false)
        .await
        .unwrap()
        .into_inner();

    let block_with_transfer = rosetta_client
        .block(&BlockRequest::by_index(chain_id, block_info.block_height.0))
        .await
        .unwrap();
    let block_with_transfer = block_with_transfer.block;
    // Verify failed txn
    let rosetta_txn = block_with_transfer
        .transactions
        .get(txn_version.saturating_sub(block_info.first_version.0) as usize)
        .unwrap();

    assert_transfer_transaction(
        sender,
        AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap(),
        TRANSFER_AMOUNT,
        actual_txn,
        rosetta_txn,
    );
}

fn assert_transfer_transaction(
    sender: AccountAddress,
    receiver: AccountAddress,
    transfer_amount: u64,
    actual_txn: &UserTransaction,
    rosetta_txn: &aptos_rosetta::types::Transaction,
) {
    // Check the transaction
    assert_eq!(
        format!("{:x}", actual_txn.info.hash),
        rosetta_txn.transaction_identifier.hash
    );

    let rosetta_txn_metadata = rosetta_txn.metadata;
    assert_eq!(TransactionType::User, rosetta_txn_metadata.transaction_type);
    assert_eq!(actual_txn.info.version.0, rosetta_txn_metadata.version.0);
    assert_eq!(rosetta_txn.operations.len(), 3);

    // Check the operations
    let mut seen_deposit = false;
    let mut seen_withdraw = false;
    for (i, operation) in rosetta_txn.operations.iter().enumerate() {
        assert_eq!(i as u64, operation.operation_identifier.index);
        if !seen_deposit && !seen_withdraw {
            match OperationType::from_str(&operation.operation_type).unwrap() {
                OperationType::Deposit => {
                    seen_deposit = true;
                    assert_deposit(
                        operation,
                        transfer_amount,
                        receiver,
                        actual_txn.info.success,
                    );
                }
                OperationType::Withdraw => {
                    seen_withdraw = true;
                    assert_withdraw(operation, transfer_amount, sender, actual_txn.info.success);
                }
                _ => panic!("Shouldn't get any other operations"),
            }
        } else if !seen_deposit {
            seen_deposit = true;
            assert_deposit(
                operation,
                transfer_amount,
                receiver,
                actual_txn.info.success,
            );
        } else if !seen_withdraw {
            seen_withdraw = true;
            assert_withdraw(operation, transfer_amount, sender, actual_txn.info.success);
        } else {
            // Gas is always last
            assert_gas(
                operation,
                actual_txn.request.gas_unit_price.0 * actual_txn.info.gas_used.0,
                sender,
                true,
            );
        }
    }
}

fn assert_deposit(
    operation: &Operation,
    expected_amount: u64,
    account: AccountAddress,
    success: bool,
) {
    assert_transfer(
        operation,
        OperationType::Deposit,
        expected_amount.to_string(),
        account,
        success,
    );
}

fn assert_withdraw(
    operation: &Operation,
    expected_amount: u64,
    account: AccountAddress,
    success: bool,
) {
    assert_transfer(
        operation,
        OperationType::Withdraw,
        format!("-{}", expected_amount),
        account,
        success,
    );
}

fn assert_gas(operation: &Operation, expected_amount: u64, account: AccountAddress, success: bool) {
    assert_transfer(
        operation,
        OperationType::Fee,
        format!("-{}", expected_amount),
        account,
        success,
    );
}

fn assert_transfer(
    operation: &Operation,
    expected_type: OperationType,
    expected_amount: String,
    account: AccountAddress,
    success: bool,
) {
    assert_eq!(expected_type.to_string(), operation.operation_type);
    let amount = operation.amount.as_ref().unwrap();
    assert_eq!(native_coin(), amount.currency);
    assert_eq!(expected_amount, amount.value);
    assert_eq!(
        &AccountIdentifier::from(account),
        operation.account.as_ref().unwrap()
    );
    let expected_status = if success {
        OperationStatusType::Success
    } else {
        OperationStatusType::Failure
    }
    .to_string();
    assert_eq!(&expected_status, operation.status.as_ref().unwrap());
}

/// Try for 2 seconds to get a response.  This handles the fact that it's starting async
async fn try_until_ok_default<F, Fut, T>(function: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    try_until_ok(
        DEFAULT_MAX_WAIT_DURATION,
        DEFAULT_INTERVAL_DURATION,
        function,
    )
    .await
}

async fn try_until_ok<F, Fut, T>(
    total_wait: Duration,
    interval: Duration,
    function: F,
) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut result = Err(anyhow::Error::msg("Failed to get response"));
    let start = Instant::now();
    while start.elapsed() < total_wait {
        result = function().await;
        if result.is_ok() {
            break;
        }
        tokio::time::sleep(interval).await;
    }

    result
}

async fn create_account_and_wait(
    rosetta_client: &RosettaClient,
    rest_client: &aptos_rest_client::Client,
    network_identifier: &NetworkIdentifier,
    sender_key: &Ed25519PrivateKey,
    new_account: AccountAddress,
    txn_expiry_duration: Duration,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<Box<UserTransaction>, Box<UserTransaction>> {
    let expiry_time = expiry_time(txn_expiry_duration);
    let txn_hash = rosetta_client
        .create_account(
            network_identifier,
            sender_key,
            new_account,
            expiry_time.as_secs(),
            sequence_number,
            max_gas,
            gas_unit_price,
        )
        .await
        .expect("Expect transfer to successfully submit to mempool")
        .hash;
    wait_for_transaction(rest_client, expiry_time, txn_hash).await
}

async fn transfer_and_wait(
    rosetta_client: &RosettaClient,
    rest_client: &aptos_rest_client::Client,
    network_identifier: &NetworkIdentifier,
    sender_key: &Ed25519PrivateKey,
    receiver: AccountAddress,
    amount: u64,
    txn_expiry_duration: Duration,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<Box<UserTransaction>, ErrorWrapper> {
    let expiry_time = expiry_time(txn_expiry_duration);
    let txn_hash = rosetta_client
        .transfer(
            network_identifier,
            sender_key,
            receiver,
            amount,
            expiry_time.as_secs(),
            sequence_number,
            max_gas,
            gas_unit_price,
        )
        .await
        .map_err(ErrorWrapper::BeforeSubmission)?
        .hash;
    wait_for_transaction(rest_client, expiry_time, txn_hash)
        .await
        .map_err(ErrorWrapper::AfterSubmission)
}

async fn wait_for_transaction(
    rest_client: &aptos_rest_client::Client,
    expiry_time: Duration,
    txn_hash: String,
) -> Result<Box<UserTransaction>, Box<UserTransaction>> {
    let hash_value = HashValue::from_str(&txn_hash).unwrap();
    let response = rest_client
        .wait_for_transaction_by_hash(hash_value, expiry_time.as_secs())
        .await;
    match response {
        Ok(response) => {
            if let Transaction::UserTransaction(txn) = response.into_inner() {
                Ok(txn)
            } else {
                panic!("Transaction is supposed to be a UserTransaction!")
            }
        }
        Err(_) => {
            if let Transaction::UserTransaction(txn) = rest_client
                .get_transaction_by_hash(hash_value)
                .await
                .unwrap()
                .into_inner()
            {
                Err(txn)
            } else {
                panic!("Failed transaction is supposed to be a UserTransaction!");
            }
        }
    }
}

fn expiry_time(txn_expiry_duration: Duration) -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .saturating_add(txn_expiry_duration)
}

#[derive(Debug)]
pub enum ErrorWrapper {
    BeforeSubmission(anyhow::Error),
    AfterSubmission(Box<UserTransaction>),
}
