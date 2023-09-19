// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    versioned_data::VersionedData, versioned_group_data::VersionedGroupData,
    versioned_modules::VersionedModules,
};
use aptos_types::{
    executable::{Executable, ModulePath},
    write_set::TransactionWrite,
};
use serde::Serialize;
use std::{fmt::Debug, hash::Hash};

pub mod types;
pub mod unsync_map;
mod utils;
pub mod versioned_data;
pub mod versioned_group_data;
pub mod versioned_modules;

#[cfg(test)]
mod unit_tests;

/// Main multi-version data-structure used by threads to read/write during parallel
/// execution.
///
/// Concurrency is managed by DashMap, i.e. when a method accesses a BTreeMap at a
/// given key, it holds exclusive access and doesn't need to explicitly synchronize
/// with other reader/writers.
///
/// TODO: separate V into different generic types for data and code modules with specialized
/// traits (currently both WriteOp for executor).
pub struct MVHashMap<K, T, V: TransactionWrite, X: Executable> {
    data: VersionedData<K, V>,
    group_data: VersionedGroupData<K, T, V>,
    modules: VersionedModules<K, V, X>,
}

impl<
        K: ModulePath + Hash + Clone + Eq + Debug,
        T: Hash + Clone + Eq + Debug + Serialize,
        V: TransactionWrite,
        X: Executable,
    > MVHashMap<K, T, V, X>
{
    // -----------------------------------
    // Functions shared for data and modules.

    pub fn new() -> MVHashMap<K, T, V, X> {
        MVHashMap {
            data: VersionedData::new(),
            group_data: VersionedGroupData::new(),
            modules: VersionedModules::new(),
        }
    }

    /// Contains 'simple' versioned data (nothing contained in groups).
    pub fn data(&self) -> &VersionedData<K, V> {
        &self.data
    }

    /// Contains data representing resource groups, or more generically, internally
    /// containing different values mapped to tags of type T.
    pub fn group_data(&self) -> &VersionedGroupData<K, T, V> {
        &self.group_data
    }

    pub fn modules(&self) -> &VersionedModules<K, V, X> {
        &self.modules
    }
}

impl<
        K: ModulePath + Hash + Clone + Debug + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
        X: Executable,
    > Default for MVHashMap<K, T, V, X>
{
    fn default() -> Self {
        Self::new()
    }
}
