// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION};
use aptos_api::Context;
use aptos_api_types::{AsConverter, LedgerInfo, Transaction, TransactionOnChainData};
use aptos_logger::prelude::*;
use aptos_vm::data_cache::StorageAdapterOwned;
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use storage_interface::state_view::DbStateView;
use tokio::task::JoinHandle;

// Default Values
const RETRY_TIME_MILLIS: u64 = 300;
const MAX_RETRY_TIME_MILLIS: u64 = 120000;
const TRANSACTION_FETCH_BATCH_SIZE: u16 = 500;
const TRANSACTION_CHANNEL_SIZE: usize = 35;

#[derive(Debug)]
pub struct Fetcher {
    pub context: Arc<Context>,
    options: TransactionFetcherOptions,
    chain_id: u8,
    current_version: u64,
    highest_known_version: u64,
    transactions_sender: mpsc::Sender<Vec<Transaction>>,
}

impl Fetcher {
    pub fn new(
        context: Arc<Context>,
        starting_version: u64,
        options: TransactionFetcherOptions,
        transactions_sender: mpsc::Sender<Vec<Transaction>>,
    ) -> Self {
        Self {
            context,
            options,
            chain_id: 0,
            current_version: starting_version,
            highest_known_version: starting_version,
            transactions_sender,
        }
    }

    pub fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let info = self.context.get_latest_ledger_info_wrapped()?;
        self.highest_known_version = info.ledger_version.0;
        self.chain_id = info.chain_id;
        Ok(())
    }

    /// Will keep looping and checking the latest ledger info to see if there are new transactions
    /// If there are, it will set the highest known version
    async fn ensure_highest_known_version(&mut self) {
        let mut empty_loops = 0;
        while self.current_version >= self.highest_known_version {
            if empty_loops > 0 {
                tokio::time::sleep(self.options.starting_retry_time).await;
            }
            empty_loops += 1;
            if let Err(err) = self.set_highest_known_version() {
                error!(
                    error = format!("{:?}", err),
                    "Failed to set highest known version"
                );
                continue;
            } else {
                sample!(
                    SampleRate::Frequency(10),
                    aptos_logger::info!(
                        highest_known_version = self.highest_known_version,
                        "Found new highest known version",
                    )
                );
            }
        }
    }

    /// Main loop for fetching transactions
    /// Fetches transactions in batches of `options.transaction_fetch_batch_size` and sends them to the processor channel.
    /// If the processor channel is full, it will wait for the processor to catch up.
    /// 1. Get the latest ledger info, and set the highest known version (if we've caught up)
    /// 2. Determine how many batches of size `options.transaction_fetch_batch_size` we need to catch up
    /// 3. Spawn tasks which fetch 'raw' `OnChainTransactions` from storage, and convert them to `Transaction`s. We spawn at most `options.max_tasks` tasks.
    /// 4. We wait for all the tasks to complete, then send the `Transaction`s to the processor, via the `transactions_sender` channel.
    pub async fn run(&mut self) {
        let transaction_fetch_batch_size = self.options.transaction_fetch_batch_size;
        loop {
            self.ensure_highest_known_version().await;

            let num_missing = self.highest_known_version - self.current_version;

            let num_batches = std::cmp::max(
                std::cmp::min(
                    (num_missing as f64 / transaction_fetch_batch_size as f64).ceil() as u64,
                    self.options.max_tasks as u64,
                ),
                1,
            ) as usize;

            info!(
                num_missing = num_missing,
                num_batches = num_batches,
                current_version = self.current_version,
                highest_known_version = self.highest_known_version,
                "Preparing to fetch transactions"
            );

            let fetch_start = chrono::Utc::now().naive_utc();
            let mut tasks = vec![];
            for i in 0..num_batches {
                let starting_version =
                    self.current_version + (i as u64 * transaction_fetch_batch_size as u64);

                let context = self.context.clone();
                let highest_known_version = self.highest_known_version;
                let task = tokio::spawn(async move {
                    fetch_nexts(
                        context,
                        starting_version,
                        highest_known_version,
                        transaction_fetch_batch_size,
                    )
                    .await
                });
                tasks.push(task);
            }

            let batches = match futures::future::try_join_all(tasks).await {
                Ok(res) => res,
                Err(err) => panic!("Error fetching transaction batches: {:?}", err),
            };

            let versions_fetched = batches.iter().fold(0, |acc, v| acc + v.len());
            let fetch_millis = (chrono::Utc::now().naive_utc() - fetch_start).num_milliseconds();
            info!(
                versions_fetched = versions_fetched,
                fetch_millis = fetch_millis,
                num_batches = num_batches,
                "Finished fetching transaction batches"
            );
            self.send_transaction_batches(batches).await;
        }
    }

    /// Sends the transaction batches to the processor via the `transactions_sender` channel
    async fn send_transaction_batches(&mut self, transaction_batches: Vec<Vec<Transaction>>) {
        let send_start = chrono::Utc::now().naive_utc();
        let num_batches = transaction_batches.len();
        let mut versions_sent: usize = 0;
        // Send keeping track of the last version sent by the batch
        for batch in transaction_batches {
            versions_sent += batch.len();
            self.current_version = std::cmp::max(
                batch.last().unwrap().version().unwrap() + 1,
                self.current_version,
            );
            self.transactions_sender
                .send(batch)
                .await
                .expect("Should be able to send transaction on channel");
        }

        let send_millis = (chrono::Utc::now().naive_utc() - send_start).num_milliseconds();
        info!(
            versions_sent = versions_sent,
            send_millis = send_millis,
            num_batches = num_batches,
            "Finished sending transaction batches"
        );
    }
}

async fn fetch_raw_txns_with_retries(
    context: Arc<Context>,
    starting_version: u64,
    ledger_version: u64,
    transaction_fetch_batch_size: u16,
    max_retries: u8,
) -> Vec<TransactionOnChainData> {
    let mut retries = 0;
    loop {
        match context.get_transactions(
            starting_version,
            transaction_fetch_batch_size,
            ledger_version,
        ) {
            Ok(raw_txns) => return raw_txns,
            Err(err) => {
                UNABLE_TO_FETCH_TRANSACTION.inc();
                retries += 1;
                if retries >= max_retries {
                    error!(
                        starting_version = starting_version,
                        transaction_fetch_batch_size = transaction_fetch_batch_size,
                        error = format!("{:?}", err),
                        "Could not fetch transactions: retries exhausted",
                    );
                    panic!(
                        "Could not fetch {} transactions after {} retries, starting at {}: {:?}",
                        transaction_fetch_batch_size, retries, starting_version, err
                    );
                } else {
                    error!(
                        starting_version = starting_version,
                        transaction_fetch_batch_size = transaction_fetch_batch_size,
                        error = format!("{:?}", err),
                        "Could not fetch transactions: will retry",
                    );
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
}

async fn fetch_nexts(
    context: Arc<Context>,
    starting_version: u64,
    ledger_version: u64,
    transaction_fetch_batch_size: u16,
) -> Vec<Transaction> {
    let start_millis = chrono::Utc::now().naive_utc();

    let raw_txns = fetch_raw_txns_with_retries(
        context.clone(),
        starting_version,
        ledger_version,
        transaction_fetch_batch_size,
        3,
    )
    .await;

    let mut timestamp = context.db.get_block_timestamp(starting_version).unwrap();

    let resolver = context.move_resolver().unwrap();
    let converter = resolver.as_converter(context.db.clone());

    let transactions_res: Result<Vec<Transaction>, anyhow::Error> = raw_txns
        .into_iter()
        .map(|t| {
            // Update the timestamp if the next block occurs
            if let aptos_types::transaction::Transaction::BlockMetadata(ref txn) = t.transaction {
                timestamp = txn.timestamp_usecs();
            }
            let txn = converter.try_into_onchain_transaction(timestamp, t)?;
            Ok(remove_null_bytes_from_txn(txn))
        })
        .collect::<Result<_, anyhow::Error>>();

    let transactions = match transactions_res {
        Ok(transactions) => transactions,
        Err(err) => {
            UNABLE_TO_FETCH_TRANSACTION.inc();
            error!(
                starting_version = starting_version,
                transaction_fetch_batch_size = transaction_fetch_batch_size,
                error = format!("{:?}", err),
                "Could not convert from OnChainTransactions",
            );
            panic!(
                "Could not convert {} txn from OnChainTransactions starting at {}: {:?}",
                transaction_fetch_batch_size, starting_version, err
            );
        }
    };

    if transactions.is_empty() {
        panic!("No transactions!");
    }

    let fetch_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();

    info!(
        starting_version = starting_version,
        num_transactions = transactions.len(),
        time_millis = fetch_millis,
        actual_last_version = transactions
            .last()
            .map(|txn| txn.version().unwrap())
            .unwrap_or(0),
        "Fetched transactions",
    );

    FETCHED_TRANSACTION.inc();

    transactions
}

#[derive(Clone, Debug)]
pub struct TransactionFetcherOptions {
    pub starting_retry_time_millis: u64,
    pub starting_retry_time: Duration,
    pub max_retry_time_millis: u64,
    pub max_retry_time: Duration,
    pub transaction_fetch_batch_size: u16,
    pub max_pending_batches: usize,
    pub max_tasks: usize,
}

fn default_if_zero<T>(value: Option<T>, default: T) -> T
where
    T: PartialEq + Copy + Default,
{
    match value {
        Some(v) => {
            if v == T::default() {
                default
            } else {
                v
            }
        }
        None => default,
    }
}

impl TransactionFetcherOptions {
    pub fn new(
        starting_retry_time_millis: Option<u64>,
        max_retry_time_millis: Option<u64>,
        transaction_fetch_batch_size: Option<u16>,
        max_pending_batches: Option<usize>,
        max_tasks: usize,
    ) -> Self {
        let starting_retry_time_millis =
            default_if_zero(starting_retry_time_millis, RETRY_TIME_MILLIS);

        let max_retry_time_millis = default_if_zero(max_retry_time_millis, MAX_RETRY_TIME_MILLIS);

        let transaction_fetch_batch_size =
            default_if_zero(transaction_fetch_batch_size, TRANSACTION_FETCH_BATCH_SIZE);

        let max_pending_batches = default_if_zero(max_pending_batches, TRANSACTION_CHANNEL_SIZE);

        TransactionFetcherOptions {
            starting_retry_time_millis,
            starting_retry_time: Duration::from_millis(starting_retry_time_millis),
            max_retry_time_millis,
            max_retry_time: Duration::from_millis(max_retry_time_millis),
            transaction_fetch_batch_size,
            max_pending_batches,
            max_tasks: std::cmp::max(max_tasks, 1),
        }
    }
}

impl Default for TransactionFetcherOptions {
    fn default() -> Self {
        TransactionFetcherOptions::new(None, None, None, None, 5)
    }
}

pub struct TransactionFetcher {
    starting_version: u64,
    options: TransactionFetcherOptions,
    pub context: Arc<Context>,
    pub resolver: Arc<StorageAdapterOwned<DbStateView>>,
    fetcher_handle: Option<JoinHandle<()>>,
    transactions_sender: Option<mpsc::Sender<Vec<Transaction>>>,
    transaction_receiver: mpsc::Receiver<Vec<Transaction>>,
}

impl TransactionFetcher {
    pub fn new(
        context: Arc<Context>,
        resolver: Arc<StorageAdapterOwned<DbStateView>>,
        starting_version: u64,
        options: TransactionFetcherOptions,
    ) -> Self {
        let (transactions_sender, transaction_receiver) =
            mpsc::channel::<Vec<Transaction>>(options.max_pending_batches);

        Self {
            starting_version,
            options,
            context,
            resolver,
            fetcher_handle: None,
            transactions_sender: Some(transactions_sender),
            transaction_receiver,
        }
    }
}

#[async_trait::async_trait]
impl TransactionFetcherTrait for TransactionFetcher {
    /// Fetches the next batch based on its internal version counter
    async fn fetch_next_batch(&mut self) -> Vec<Transaction> {
        self.transaction_receiver
            .next()
            .await
            .expect("No transactions, producer of batches died")
    }

    fn fetch_ledger_info(&mut self) -> LedgerInfo {
        self.context
            .get_latest_ledger_info_wrapped()
            .unwrap_or_else(|err| panic!("Failed to get ledger info: {}", err))
    }

    async fn set_version(&mut self, version: u64) {
        if self.fetcher_handle.is_some() {
            panic!("TransactionFetcher already started!");
        }
        self.starting_version = version;
    }

    async fn start(&mut self) {
        if self.fetcher_handle.is_some() {
            panic!("TransactionFetcher already started!");
        }
        let context = self.context.clone();
        let transactions_sender = self.transactions_sender.take().unwrap();
        let starting_version = self.starting_version;

        let options2 = self.options.clone();
        let fetcher_handle = tokio::spawn(async move {
            let mut fetcher =
                Fetcher::new(context, starting_version, options2, transactions_sender);
            fetcher.run().await;
        });
        self.fetcher_handle = Some(fetcher_handle);
    }
}

pub fn string_null_byte_replacement(value: &mut str) -> String {
    value.replace('\u{0000}', "").replace("\\u0000", "")
}

pub fn recurse_remove_null_bytes_from_json(sub_json: &mut Value) {
    match sub_json {
        Value::Array(array) => {
            for item in array {
                recurse_remove_null_bytes_from_json(item);
            }
        }
        Value::Object(object) => {
            for (_key, value) in object {
                recurse_remove_null_bytes_from_json(value);
            }
        }
        Value::String(str) => {
            if !str.is_empty() {
                let replacement = string_null_byte_replacement(str);
                *str = replacement;
            }
        }
        _ => {}
    }
}

pub fn remove_null_bytes_from_txn(txn: Transaction) -> Transaction {
    let mut txn_json = serde_json::to_value(txn).unwrap();
    recurse_remove_null_bytes_from_json(&mut txn_json);
    serde_json::from_value::<Transaction>(txn_json).unwrap()
}

pub fn remove_null_bytes_from_txns(txns: Vec<Transaction>) -> Vec<Transaction> {
    txns.into_iter()
        .map(remove_null_bytes_from_txn)
        .collect::<Vec<Transaction>>()
}

/// For mocking TransactionFetcher in tests
#[async_trait::async_trait]
pub trait TransactionFetcherTrait: Send + Sync {
    async fn fetch_next_batch(&mut self) -> Vec<Transaction>;

    fn fetch_ledger_info(&mut self) -> LedgerInfo;

    async fn set_version(&mut self, version: u64);

    async fn start(&mut self);
}
