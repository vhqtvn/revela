// Copyright © Aptos Foundation

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::delegator_pools::{DelegatorPool, DelegatorPoolBalanceMetadata};
use crate::{schema::current_delegator_balances, util::standardize_address};
use anyhow::Context;
use aptos_api_types::{
    DeleteTableItem as APIDeleteTableItem, Transaction as APITransaction,
    WriteResource as APIWriteResource, WriteSetChange as APIWriteSetChange,
    WriteTableItem as APIWriteTableItem,
};
use bigdecimal::{BigDecimal, Zero};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type TableHandle = String;
pub type Address = String;
pub type ActiveShareMapping = HashMap<TableHandle, DelegatorPoolBalanceMetadata>;
pub type CurrentDelegatorBalancePK = (Address, Address, String);
pub type CurrentDelegatorBalanceMap = HashMap<CurrentDelegatorBalancePK, CurrentDelegatorBalance>;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(delegator_address, pool_address, pool_type))]
#[diesel(table_name = current_delegator_balances)]
pub struct CurrentDelegatorBalance {
    pub delegator_address: String,
    pub pool_address: String,
    pub pool_type: String,
    pub table_handle: String,
    pub last_transaction_version: i64,
    pub shares: BigDecimal,
}

impl CurrentDelegatorBalance {
    /// We're only indexing active_shares for now because that's all the UI needs and indexing
    /// the inactive_shares or pending_withdrawal_shares would be more complicated.
    pub fn from_write_table_item(
        write_table_item: &APIWriteTableItem,
        txn_version: i64,
        active_share_mapping: &ActiveShareMapping,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&write_table_item.handle.to_string());
        // The mapping will tell us if the table item is an active share table
        if let Some(pool_balance) = active_share_mapping.get(&table_handle) {
            let pool_address = pool_balance.staking_pool_address.clone();
            let delegator_address = standardize_address(&write_table_item.key.to_string());
            let data = write_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "This table item should be an active share item, table_item {:?}, version {}",
                    write_table_item, txn_version
                )
            });
            let shares = data
                .value
                .as_str()
                .map(|s| s.parse::<BigDecimal>())
                .context(format!(
                    "value is not a string: {:?}, table_item {:?}, version {}",
                    data.value, write_table_item, txn_version
                ))?
                .context(format!(
                    "cannot parse string as u64: {:?}, version {}",
                    data.value, txn_version
                ))?;
            let shares = shares / &pool_balance.scaling_factor;
            Ok(Some(Self {
                delegator_address,
                pool_address,
                pool_type: "active_shares".to_string(),
                table_handle,
                last_transaction_version: txn_version,
                shares,
            }))
        } else {
            Ok(None)
        }
    }

    // Setting amount to 0 if table item is deleted
    pub fn from_delete_table_item(
        delete_table_item: &APIDeleteTableItem,
        txn_version: i64,
        active_share_mapping: &ActiveShareMapping,
    ) -> anyhow::Result<Option<Self>> {
        let table_handle = standardize_address(&delete_table_item.handle.to_string());
        // The mapping will tell us if the table item is an active share table
        if let Some(pool_balance) = active_share_mapping.get(&table_handle) {
            let delegator_address = standardize_address(&delete_table_item.key.to_string());

            return Ok(Some(Self {
                delegator_address,
                pool_address: pool_balance.staking_pool_address.clone(),
                pool_type: "active_shares".to_string(),
                table_handle,
                last_transaction_version: txn_version,
                shares: BigDecimal::zero(),
            }));
        }
        Ok(None)
    }

    pub fn get_active_share_map(
        write_resource: &APIWriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<ActiveShareMapping>> {
        if let Some(balance) = DelegatorPool::get_balance_metadata(write_resource, txn_version)? {
            Ok(Some(HashMap::from([(
                balance.active_share_table_handle.clone(),
                balance,
            )])))
        } else {
            Ok(None)
        }
    }

    pub fn from_transaction(
        transaction: &APITransaction,
    ) -> anyhow::Result<CurrentDelegatorBalanceMap> {
        let mut active_share_mapping: ActiveShareMapping = HashMap::new();
        let mut current_delegator_balances: CurrentDelegatorBalanceMap = HashMap::new();
        // Do a first pass to get the mapping of active_share table handles to staking pool resource
        if let APITransaction::UserTransaction(user_txn) = transaction {
            let txn_version = user_txn.info.version.0 as i64;
            for wsc in &user_txn.info.changes {
                if let APIWriteSetChange::WriteResource(write_resource) = wsc {
                    let maybe_map =
                        Self::get_active_share_map(write_resource, txn_version).unwrap();
                    if let Some(map) = maybe_map {
                        active_share_mapping.extend(map);
                    }
                }
            }
            // Now make a pass through table items to get the actual delegator balances
            for wsc in &user_txn.info.changes {
                let txn_version = user_txn.info.version.0 as i64;
                let maybe_delegator_balance = match wsc {
                    APIWriteSetChange::DeleteTableItem(table_item) => {
                        Self::from_delete_table_item(table_item, txn_version, &active_share_mapping)
                            .unwrap()
                    },
                    APIWriteSetChange::WriteTableItem(table_item) => {
                        Self::from_write_table_item(table_item, txn_version, &active_share_mapping)
                            .unwrap()
                    },
                    _ => None,
                };
                if let Some(delegator_balance) = maybe_delegator_balance {
                    current_delegator_balances.insert(
                        (
                            delegator_balance.delegator_address.clone(),
                            delegator_balance.pool_address.clone(),
                            delegator_balance.pool_type.clone(),
                        ),
                        delegator_balance,
                    );
                }
            }
        }
        Ok(current_delegator_balances)
    }
}
