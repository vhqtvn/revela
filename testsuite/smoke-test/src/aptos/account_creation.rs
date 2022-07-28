// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};

pub struct AccountCreation;

impl Test for AccountCreation {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::account-creation"
    }
}

#[async_trait::async_trait]
impl AptosTest for AccountCreation {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        // created by root account
        let mut accounts = vec![];
        for _ in 0..10 {
            let local_account = ctx.random_account();
            ctx.create_user_account(local_account.public_key()).await?;
            ctx.mint(local_account.address(), 10000).await?;
            accounts.push(local_account);
        }
        // created by user account
        for account in &mut accounts {
            let new_account = ctx.random_account();
            let txn = account.sign_with_transaction_builder(
                ctx.aptos_transaction_factory()
                    .payload(aptos_stdlib::account_create_account(new_account.address())),
            );
            ctx.client().submit_and_wait(&txn).await?;
        }
        // create and fund
        for mut account in accounts {
            let new_account = ctx.random_account();
            let txn = account.sign_with_transaction_builder(
                ctx.aptos_transaction_factory()
                    .payload(aptos_stdlib::account_transfer(new_account.address(), 5000)),
            );
            ctx.client().submit_and_wait(&txn).await?;
            assert_eq!(ctx.get_balance(new_account.address()).await.unwrap(), 5000);
        }
        Ok(())
    }
}
