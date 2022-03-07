// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use anyhow::bail;
use aptos_config::config::NodeConfig;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{transaction_builder::Currency, types::LocalAccount};
use aptos_types::account_address::AccountAddress;
use forge::{NetworkContext, NetworkTest, NodeExt, Result, Test};
use tokio::runtime::Runtime;

#[derive(Debug)]
pub struct LaunchFullnode;

impl Test for LaunchFullnode {
    fn name(&self) -> &'static str {
        "smoke-test:launch-fullnode"
    }
}

impl NetworkTest for LaunchFullnode {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(self.async_run(ctx))
    }
}

impl LaunchFullnode {
    async fn async_run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let version = ctx.swarm().versions().max().unwrap();
        let fullnode_peer_id = ctx
            .swarm()
            .add_full_node(&version, NodeConfig::default_for_public_full_node())?;

        let fullnode = ctx.swarm().full_node_mut(fullnode_peer_id).unwrap();
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(10))
            .await?;

        let client = fullnode.rest_client();

        let factory = ctx.swarm().chain_info().transaction_factory();
        let mut account1 = LocalAccount::generate(ctx.core().rng());
        let account2 = LocalAccount::generate(ctx.core().rng());

        ctx.swarm()
            .chain_info()
            .create_parent_vasp_account(Currency::XUS, account1.authentication_key())
            .await?;
        ctx.swarm()
            .chain_info()
            .fund(Currency::XUS, account1.address(), 100)
            .await?;
        ctx.swarm()
            .chain_info()
            .create_parent_vasp_account(Currency::XUS, account2.authentication_key())
            .await?;

        wait_for_account(&client, account1.address()).await?;

        let txn = account1.sign_with_transaction_builder(factory.peer_to_peer(
            Currency::XUS,
            account2.address(),
            10,
        ));

        client.submit_and_wait(&txn).await?;
        let balances = client
            .get_account_balances(account1.address())
            .await?
            .into_inner();

        assert_eq!(
            vec![(90, "XUS".to_string())],
            balances
                .into_iter()
                .map(|b| (b.amount, b.currency_code()))
                .collect::<Vec<(u64, String)>>()
        );

        Ok(())
    }
}

async fn wait_for_account(client: &RestClient, address: AccountAddress) -> Result<()> {
    const DEFAULT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < DEFAULT_WAIT_TIMEOUT {
        if client.get_account(address).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    bail!("wait for account(address={}) timeout", address,)
}
