// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer::{
    database::{new_db_pool, PgDbPool, PgPoolConnection},
    default_processor::DefaultTransactionProcessor,
    indexer::tailer::Tailer,
    models::transactions::TransactionModel,
    token_processor::TokenTransactionProcessor,
};
use aptos_rest_client::Client;
use aptos_sdk::types::LocalAccount;
use cached_framework_packages::aptos_stdlib::aptos_token_stdlib;
use diesel::connection::Connection;
use forge::{AptosContext, AptosTest, Result, Test};
use std::sync::Arc;

pub struct Indexer;

impl Test for Indexer {
    fn name(&self) -> &'static str {
        "ecosystem::indexer"
    }
}

pub fn wipe_database(conn: &PgPoolConnection) {
    for table in [
        "metadatas",
        "token_activities",
        "token_datas",
        "token_propertys",
        "collections",
        "ownerships",
        "write_set_changes",
        "events",
        "user_transactions",
        "block_metadata_transactions",
        "transactions",
        "processor_statuses",
        "ledger_infos",
        "__diesel_schema_migrations",
    ] {
        conn.execute(&format!("DROP TABLE IF EXISTS {}", table))
            .unwrap();
    }
}

/// By default, skips test unless `INDEXER_DATABASE_URL` is set.
/// In CI, will explode if `INDEXER_DATABASE_URL` is NOT set.
pub fn should_skip() -> bool {
    if std::env::var("CIRCLECI").is_ok() {
        std::env::var("INDEXER_DATABASE_URL").expect("must set 'INDEXER_DATABASE_URL' in CI!");
    }
    if std::env::var("INDEXER_DATABASE_URL").is_ok() {
        false
    } else {
        println!("`INDEXER_DATABASE_URL` is not set: skipping indexer tests");
        true
    }
}

pub fn setup_indexer(ctx: &mut AptosContext) -> anyhow::Result<(PgDbPool, Tailer)> {
    let database_url = std::env::var("INDEXER_DATABASE_URL")
        .expect("must set 'INDEXER_DATABASE_URL' to run tests!");

    let conn_pool = new_db_pool(database_url.as_str())?;
    wipe_database(&conn_pool.get()?);
    let mut tailer = Tailer::new(ctx.url(), conn_pool.clone())?;
    tailer.run_migrations();

    let pg_transaction_processor = DefaultTransactionProcessor::new(conn_pool.clone());
    let token_processor = TokenTransactionProcessor::new(conn_pool.clone(), false);
    tailer.add_processor(Arc::new(pg_transaction_processor));
    tailer.add_processor(Arc::new(token_processor));
    Ok((conn_pool, tailer))
}

pub async fn execute_nft_txns<'t>(
    mut creator: LocalAccount,
    ctx: &mut AptosContext<'t>,
    client: &Client,
) -> Result<()> {
    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    let collection_builder =
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_create_collection_script(
                collection_name.clone(),
                "description".to_owned().into_bytes(),
                "uri".to_owned().into_bytes(),
                20_000_000,
                vec![false, false, false],
            ));

    let collection_txn = creator.sign_with_transaction_builder(collection_builder);
    client.submit_and_wait(&collection_txn).await?;

    let token_builder =
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_create_token_script(
                collection_name.clone(),
                token_name.clone(),
                "collection description".to_owned().into_bytes(),
                3,
                4,
                "uri".to_owned().into_bytes(),
                creator.address(),
                0,
                0,
                vec![false, false, false, false, true],
                vec!["age".as_bytes().to_vec()],
                vec!["3".as_bytes().to_vec()],
                vec!["int".as_bytes().to_vec()],
            ));

    let token_txn = creator.sign_with_transaction_builder(token_builder);
    client.submit_and_wait(&token_txn).await?;

    let token_mutator =
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_mutate_token_properties(
                creator.address(),
                creator.address(),
                collection_name.clone(),
                token_name.clone(),
                0,
                2,
                vec!["age".as_bytes().to_vec()],
                vec!["2".as_bytes().to_vec()],
                vec!["int".as_bytes().to_vec()],
            ));
    let mutate_txn = creator.sign_with_transaction_builder(token_mutator);
    client.submit_and_wait(&mutate_txn).await?;
    Ok(())
}

#[async_trait::async_trait]
impl AptosTest for Indexer {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        if aptos_indexer::should_skip_pg_tests() {
            return Ok(());
        }
        let (conn_pool, mut tailer) = setup_indexer(ctx)?;

        let client = ctx.client();
        client.get_ledger_information().await.unwrap();

        // Set up accounts, generate some traffic
        // TODO(Gas): double check this
        let mut account1 = ctx.create_and_fund_user_account(100_000_000).await.unwrap();
        let account2 = ctx.create_and_fund_user_account(100_000_000).await.unwrap();
        // This transfer should emit events
        let t_tx = ctx.transfer(&mut account1, &account2, 717).await.unwrap();
        // test NFT creation event indexing
        execute_nft_txns(account1, ctx, &client).await.unwrap();

        // Why do this twice? To ensure the idempotency of the tailer :-)
        let mut version: u64 = 0;
        for _ in 0..2 {
            // Process the next versions
            version = client
                .get_ledger_information()
                .await
                .unwrap()
                .into_inner()
                .version;
            tailer.process_next_batch((version + 1) as u8).await;

            // Get them into the array and sort by type in order to prevent ordering from breaking tests
            let mut transactions = vec![];
            for v in 0..2 {
                transactions.push(TransactionModel::get_by_version(v, &conn_pool.get()?).unwrap());
            }
            transactions.sort_by(|a, b| a.0.type_.partial_cmp(&b.0.type_).unwrap());

            // This is a block metadata transaction
            let (tx1, ut1, bmt1, events1, wsc1) = &transactions[0];
            assert_eq!(tx1.type_, "block_metadata_transaction");
            assert!(ut1.is_none());
            assert!(bmt1.is_some());
            assert!(!events1.is_empty());
            assert!(!wsc1.is_empty());

            // This is the genesis transaction
            let (tx0, ut0, bmt0, events0, wsc0) = &transactions[1];
            assert_eq!(tx0.type_, "genesis_transaction");
            assert!(ut0.is_none());
            assert!(bmt0.is_none());
            assert!(!events0.is_empty());
            assert!(wsc0.len() > 10);

            // This is the transfer
            let (tx2, ut2, bmt2, events2, wsc2) =
                TransactionModel::get_by_hash(t_tx.hash.to_string().as_str(), &conn_pool.get()?)
                    .unwrap();

            assert_eq!(tx2.type_, "user_transaction");
            assert_eq!(tx2.hash, t_tx.hash.to_string());

            // This is a user transaction, so the bmt should be None
            assert!(ut2.is_some());
            assert!(bmt2.is_none());
            assert!(wsc2.len() > 1);
            assert_eq!(events2.len(), 2);
            assert_eq!(events2.get(0).unwrap().type_, "0x1::coin::WithdrawEvent");
            assert_eq!(events2.get(1).unwrap().type_, "0x1::coin::DepositEvent");
        }

        let latest_version = tailer.set_fetcher_to_lowest_processor_version().await;
        assert!(latest_version > version);

        Ok(())
    }
}
