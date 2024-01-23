// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{publishing::publish_util::Package, ReliableTransactionSubmitter};
use crate::{
    create_account_transaction, publishing::publish_util::PackageHandler, TransactionGenerator,
    TransactionGeneratorCreator,
};
use aptos_logger::info;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use async_trait::async_trait;
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use std::sync::Arc;

// Fn + Send + Sync, as it will be called from multiple threads simultaneously
// if you need any coordination, use Arc<RwLock<X>> fields
pub type TransactionGeneratorWorker = dyn Fn(
        &LocalAccount,
        &Package,
        &LocalAccount,
        &TransactionFactory,
        &mut StdRng,
    ) -> Option<SignedTransaction>
    + Send
    + Sync;

#[async_trait]
pub trait UserModuleTransactionGenerator: Sync + Send {
    /// Called for each instance of the module we publish,
    /// if any additional transactions are needed to setup the package.
    /// For example, if we need to create an NFT collection, or otherwise
    /// call directly additional initialization of the module.
    fn initialize_package(
        &mut self,
        package: &Package,
        publisher: &mut LocalAccount,
        txn_factory: &TransactionFactory,
        rng: &mut StdRng,
    ) -> Vec<SignedTransaction>;

    /// Create TransactionGeneratorWorker function, which will be called
    /// to generate transactions to submit.
    /// TransactionGeneratorWorker will be called from multiple threads simultaneously.
    /// if you need any coordination, use Arc<RwLock<X>> fields
    /// If you need to send any additional initialization transactions
    /// (like creating and funding additional accounts), you can do so by using provided txn_executor
    async fn create_generator_fn(
        &self,
        root_account: &mut LocalAccount,
        txn_factory: &TransactionFactory,
        txn_executor: &dyn ReliableTransactionSubmitter,
        rng: &mut StdRng,
    ) -> Arc<TransactionGeneratorWorker>;
}

pub struct CustomModulesDelegationGenerator {
    rng: StdRng,
    txn_factory: TransactionFactory,
    packages: Arc<Vec<(Package, LocalAccount)>>,
    txn_generator: Arc<TransactionGeneratorWorker>,
}

impl CustomModulesDelegationGenerator {
    pub fn new(
        rng: StdRng,
        txn_factory: TransactionFactory,
        packages: Arc<Vec<(Package, LocalAccount)>>,
        txn_generator: Arc<TransactionGeneratorWorker>,
    ) -> Self {
        Self {
            rng,
            txn_factory,
            packages,
            txn_generator,
        }
    }
}

impl TransactionGenerator for CustomModulesDelegationGenerator {
    fn generate_transactions(
        &mut self,
        account: &LocalAccount,
        num_to_create: usize,
    ) -> Vec<SignedTransaction> {
        let mut requests = Vec::with_capacity(num_to_create);

        for _ in 0..num_to_create {
            let (package, publisher) = self.packages.choose(&mut self.rng).unwrap();
            let request = (self.txn_generator)(
                account,
                package,
                publisher,
                &self.txn_factory,
                &mut self.rng,
            );
            if let Some(request) = request {
                requests.push(request);
            }
        }
        requests
    }
}

pub struct CustomModulesDelegationGeneratorCreator {
    txn_factory: TransactionFactory,
    packages: Arc<Vec<(Package, LocalAccount)>>,
    txn_generator: Arc<TransactionGeneratorWorker>,
}

impl CustomModulesDelegationGeneratorCreator {
    #[allow(dead_code)]
    pub fn new_raw(
        txn_factory: TransactionFactory,
        packages: Arc<Vec<(Package, LocalAccount)>>,
        txn_generator: Arc<TransactionGeneratorWorker>,
    ) -> Self {
        Self {
            txn_factory,
            packages,
            txn_generator,
        }
    }

    pub async fn new(
        txn_factory: TransactionFactory,
        init_txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        package_name: &str,
        workload: &mut dyn UserModuleTransactionGenerator,
    ) -> Self {
        let mut packages = Self::publish_package(
            init_txn_factory.clone(),
            root_account,
            txn_executor,
            num_modules,
            package_name,
            None,
        )
        .await;
        let worker = Self::create_worker(
            init_txn_factory,
            root_account,
            txn_executor,
            &mut packages,
            workload,
        )
        .await;
        Self {
            txn_factory,
            packages: Arc::new(packages),
            txn_generator: worker,
        }
    }

    pub async fn create_worker(
        init_txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        txn_executor: &dyn ReliableTransactionSubmitter,
        packages: &mut Vec<(Package, LocalAccount)>,
        workload: &mut dyn UserModuleTransactionGenerator,
    ) -> Arc<TransactionGeneratorWorker> {
        let mut rng = StdRng::from_entropy();
        let mut requests_initialize = Vec::with_capacity(packages.len());

        for (package, publisher) in packages.iter_mut() {
            requests_initialize.append(&mut workload.initialize_package(
                package,
                publisher,
                &init_txn_factory,
                &mut rng,
            ));
        }

        if !requests_initialize.is_empty() {
            info!(
                "Initializing workload with {} transactions",
                requests_initialize.len()
            );
            txn_executor
                .execute_transactions(&requests_initialize)
                .await
                .unwrap();
        }

        info!("Done preparing workload for {} packages", packages.len());

        workload
            .create_generator_fn(root_account, &init_txn_factory, txn_executor, &mut rng)
            .await
    }

    pub async fn publish_package(
        init_txn_factory: TransactionFactory,
        root_account: &mut LocalAccount,
        txn_executor: &dyn ReliableTransactionSubmitter,
        num_modules: usize,
        package_name: &str,
        publisher_balance: Option<u64>,
    ) -> Vec<(Package, LocalAccount)> {
        let mut rng = StdRng::from_entropy();
        let mut requests_create = Vec::with_capacity(num_modules);
        let mut requests_publish = Vec::with_capacity(num_modules);
        let mut package_handler = PackageHandler::new(package_name);
        let mut packages = Vec::new();
        for _i in 0..num_modules {
            let publisher = LocalAccount::generate(&mut rng);
            let publisher_address = publisher.address();
            requests_create.push(create_account_transaction(
                root_account,
                publisher_address,
                &init_txn_factory,
                publisher_balance.unwrap_or(
                    2 * init_txn_factory.get_gas_unit_price()
                        * init_txn_factory.get_max_gas_amount(),
                ),
            ));

            let package = package_handler.pick_package(&mut rng, publisher.address());

            requests_publish.push(publisher.sign_with_transaction_builder(
                init_txn_factory.payload(package.publish_transaction_payload()),
            ));

            packages.push((package, publisher));
        }
        info!("Creating {} publisher accounts", requests_create.len());
        txn_executor
            .execute_transactions(&requests_create)
            .await
            .unwrap();

        info!("Publishing {} packages", requests_publish.len());
        txn_executor
            .execute_transactions(&requests_publish)
            .await
            .unwrap();

        info!("Done publishing {} packages", packages.len());

        packages
    }
}

impl TransactionGeneratorCreator for CustomModulesDelegationGeneratorCreator {
    fn create_transaction_generator(&self) -> Box<dyn TransactionGenerator> {
        Box::new(CustomModulesDelegationGenerator::new(
            StdRng::from_entropy(),
            self.txn_factory.clone(),
            self.packages.clone(),
            self.txn_generator.clone(),
        ))
    }
}
