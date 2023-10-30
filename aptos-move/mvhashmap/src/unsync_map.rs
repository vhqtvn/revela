// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::{GroupReadResult, MVModulesOutput},
    utils::module_hash,
};
use anyhow::bail;
use aptos_aggregator::types::DelayedFieldValue;
use aptos_crypto::hash::HashValue;
use aptos_types::{
    executable::{Executable, ExecutableDescriptor, ModulePath},
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_types::resource_group_adapter::group_size_as_sum;
use move_core_types::value::MoveTypeLayout;
use serde::Serialize;
use std::{cell::RefCell, collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

/// UnsyncMap is designed to mimic the functionality of MVHashMap for sequential execution.
/// In this case only the latest recorded version is relevant, simplifying the implementation.
/// The functionality also includes Executable caching based on the hash of ExecutableDescriptor
/// (i.e. module hash for modules published during the latest block - not at storage version).
pub struct UnsyncMap<
    K: ModulePath,
    T: Hash + Clone + Debug + Eq + Serialize,
    V: TransactionWrite,
    X: Executable,
    I: Copy,
> {
    // Only use Arc to provide unified interfaces with the MVHashMap / concurrent setting. This
    // simplifies the trait-based integration for executable caching. TODO: better representation.
    // Optional hash can store the hash of the module to avoid re-computations.
    map: RefCell<HashMap<K, (Arc<V>, Option<HashValue>, Option<Arc<MoveTypeLayout>>)>>,
    group_cache: RefCell<HashMap<K, HashMap<T, Arc<V>>>>,
    executable_cache: RefCell<HashMap<HashValue, Arc<X>>>,
    executable_bytes: RefCell<usize>,
    delayed_field_map: RefCell<HashMap<I, DelayedFieldValue>>,
}

impl<
        K: ModulePath + Hash + Clone + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
        X: Executable,
        I: Hash + Clone + Copy + Eq,
    > Default for UnsyncMap<K, T, V, X, I>
{
    fn default() -> Self {
        Self {
            map: RefCell::new(HashMap::new()),
            group_cache: RefCell::new(HashMap::new()),
            executable_cache: RefCell::new(HashMap::new()),
            executable_bytes: RefCell::new(0),
            delayed_field_map: RefCell::new(HashMap::new()),
        }
    }
}

impl<
        K: ModulePath + Hash + Clone + Eq,
        T: Hash + Clone + Debug + Eq + Serialize,
        V: TransactionWrite,
        X: Executable,
        I: Hash + Clone + Copy + Eq,
    > UnsyncMap<K, T, V, X, I>
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_group_base_values(
        &self,
        group_key: K,
        base_values: impl IntoIterator<Item = (T, V)>,
    ) {
        assert!(
            self.group_cache
                .borrow_mut()
                .insert(
                    group_key,
                    base_values
                        .into_iter()
                        .map(|(t, v)| (t, Arc::new(v)))
                        .collect()
                )
                .is_none(),
            "UnsyncMap group cache must be empty to provide base values"
        );
    }

    pub fn get_group_size(&self, group_key: &K) -> anyhow::Result<GroupReadResult> {
        Ok(match self.group_cache.borrow().get(group_key) {
            Some(group_map) => GroupReadResult::Size(group_size_as_sum(
                group_map
                    .iter()
                    .flat_map(|(t, v)| v.bytes().map(|bytes| (t, bytes))),
            )?),
            None => GroupReadResult::Uninitialized,
        })
    }

    pub fn get_value_from_group(&self, group_key: &K, value_tag: &T) -> GroupReadResult {
        self.group_cache.borrow().get(group_key).map_or(
            GroupReadResult::Uninitialized,
            |group_map| {
                GroupReadResult::Value(
                    group_map.get(value_tag).and_then(|v| v.extract_raw_bytes()),
                    // TODO[agg_v2]: support layouts.
                    None,
                )
            },
        )
    }

    /// Contains the latest group ops for the given group key.
    pub fn finalize_group(&self, group_key: &K) -> Vec<(T, Arc<V>)> {
        self.group_cache
            .borrow()
            .get(group_key)
            .expect("Resource group must be cached")
            .clone()
            .into_iter()
            .collect()
    }

    pub fn insert_group_op(&self, group_key: &K, value_tag: T, v: V) -> anyhow::Result<()> {
        use std::collections::hash_map::Entry::*;
        use WriteOpKind::*;

        match (
            self.group_cache
                .borrow_mut()
                .get_mut(group_key)
                .expect("Resource group must be cached")
                .entry(value_tag.clone()),
            v.write_op_kind(),
        ) {
            (Occupied(entry), Deletion) => {
                entry.remove();
            },
            (Occupied(mut entry), Modification) => {
                entry.insert(Arc::new(v));
            },
            (Vacant(entry), Creation) => {
                entry.insert(Arc::new(v));
            },
            (_, _) => {
                bail!(
                    "WriteOp kind {:?} not consistent with previous value at tag {:?}",
                    v.write_op_kind(),
                    value_tag
                );
            },
        }

        Ok(())
    }

    pub fn fetch_data(&self, key: &K) -> Option<(Arc<V>, Option<Arc<MoveTypeLayout>>)> {
        self.map
            .borrow()
            .get(key)
            .map(|entry| (entry.0.clone(), entry.2.clone()))
    }

    pub fn fetch_module(&self, key: &K) -> Option<MVModulesOutput<V, X>> {
        use MVModulesOutput::*;
        debug_assert!(key.module_path().is_some());

        self.map.borrow_mut().get_mut(key).map(|entry| {
            let hash = entry.1.get_or_insert(module_hash(entry.0.as_ref()));

            self.executable_cache.borrow().get(hash).map_or_else(
                || Module((entry.0.clone(), *hash)),
                |x| Executable((x.clone(), ExecutableDescriptor::Published(*hash))),
            )
        })
    }

    pub fn fetch_delayed_field(&self, id: &I) -> Option<DelayedFieldValue> {
        self.delayed_field_map.borrow().get(id).cloned()
    }

    pub fn write(&self, key: K, value: V, layout: Option<Arc<MoveTypeLayout>>) {
        self.map
            .borrow_mut()
            .insert(key, (Arc::new(value), None, layout));
    }

    /// We return false if the executable was already stored, as this isn't supposed to happen
    /// during sequential execution (and the caller may choose to e.g. log a message).
    /// Versioned modules storage does not cache executables at storage version, hence directly
    /// the descriptor hash in ExecutableDescriptor::Published is provided.
    pub fn store_executable(&self, descriptor_hash: HashValue, executable: X) -> bool {
        let size = executable.size_bytes();
        if self
            .executable_cache
            .borrow_mut()
            .insert(descriptor_hash, Arc::new(executable))
            .is_some()
        {
            *self.executable_bytes.borrow_mut() += size;
            true
        } else {
            false
        }
    }

    pub fn executable_size(&self) -> usize {
        *self.executable_bytes.borrow()
    }

    pub fn write_delayed_field(&self, id: I, value: DelayedFieldValue) {
        self.delayed_field_map.borrow_mut().insert(id, value);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::test::{KeyType, TestValue};
    use aptos_types::executable::ExecutableTestType;
    use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some_eq};

    fn finalize_group_as_hashmap(
        map: &UnsyncMap<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>,
        key: &KeyType<Vec<u8>>,
    ) -> HashMap<usize, Arc<TestValue>> {
        map.finalize_group(key).into_iter().collect()
    }

    #[test]
    fn group_commit_idx() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>::new();

        map.set_group_base_values(
            ap.clone(),
            // base tag 1, 2, 3
            (1..4).map(|i| (i, TestValue::with_kind(i, true))),
        );
        assert_ok!(map.insert_group_op(&ap, 2, TestValue::with_kind(202, false)));
        assert_ok!(map.insert_group_op(&ap, 3, TestValue::with_kind(203, false)));
        let committed = finalize_group_as_hashmap(&map, &ap);

        // // The value at tag 1 is from base, while 2 and 3 are from txn 3.
        // // (Arc compares with value equality)
        assert_eq!(committed.len(), 3);
        assert_some_eq!(committed.get(&1), &Arc::new(TestValue::with_kind(1, true)));
        assert_some_eq!(
            committed.get(&2),
            &Arc::new(TestValue::with_kind(202, false))
        );
        assert_some_eq!(
            committed.get(&3),
            &Arc::new(TestValue::with_kind(203, false))
        );

        assert_ok!(map.insert_group_op(&ap, 3, TestValue::with_kind(303, false)));
        assert_ok!(map.insert_group_op(&ap, 4, TestValue::with_kind(304, true)));
        let committed = finalize_group_as_hashmap(&map, &ap);
        assert_eq!(committed.len(), 4);
        assert_some_eq!(committed.get(&1), &Arc::new(TestValue::with_kind(1, true)));
        assert_some_eq!(
            committed.get(&2),
            &Arc::new(TestValue::with_kind(202, false))
        );
        assert_some_eq!(
            committed.get(&3),
            &Arc::new(TestValue::with_kind(303, false))
        );
        assert_some_eq!(
            committed.get(&4),
            &Arc::new(TestValue::with_kind(304, true))
        );

        assert_ok!(map.insert_group_op(&ap, 0, TestValue::with_kind(100, true)));
        assert_ok!(map.insert_group_op(&ap, 1, TestValue::deletion()));
        assert_err!(map.insert_group_op(&ap, 1, TestValue::deletion()));
        let committed = finalize_group_as_hashmap(&map, &ap);
        assert_eq!(committed.len(), 4);
        assert_some_eq!(
            committed.get(&0),
            &Arc::new(TestValue::with_kind(100, true))
        );
        assert_none!(committed.get(&1));
        assert_some_eq!(
            committed.get(&2),
            &Arc::new(TestValue::with_kind(202, false))
        );
        assert_some_eq!(
            committed.get(&3),
            &Arc::new(TestValue::with_kind(303, false))
        );
        assert_some_eq!(
            committed.get(&4),
            &Arc::new(TestValue::with_kind(304, true))
        );

        assert_ok!(map.insert_group_op(&ap, 0, TestValue::deletion()));
        assert_ok!(map.insert_group_op(&ap, 1, TestValue::with_kind(400, true)));
        assert_ok!(map.insert_group_op(&ap, 2, TestValue::deletion()));
        assert_ok!(map.insert_group_op(&ap, 3, TestValue::deletion()));
        assert_ok!(map.insert_group_op(&ap, 4, TestValue::deletion()));
        let committed = finalize_group_as_hashmap(&map, &ap);
        assert_eq!(committed.len(), 1);
        assert_some_eq!(
            committed.get(&1),
            &Arc::new(TestValue::with_kind(400, true))
        );
    }

    #[should_panic]
    #[test]
    fn set_base_twice() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>::new();

        map.set_group_base_values(
            ap.clone(),
            (1..4).map(|i| (i, TestValue::with_kind(i, true))),
        );
        map.set_group_base_values(
            ap.clone(),
            (1..4).map(|i| (i, TestValue::with_kind(i, true))),
        );
    }

    #[should_panic]
    #[test]
    fn group_op_without_base() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>::new();

        assert_ok!(map.insert_group_op(&ap, 3, TestValue::with_kind(10, true)));
    }

    #[should_panic]
    #[test]
    fn group_no_path_exists() {
        let ap = KeyType(b"/foo/b".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>::new();

        map.finalize_group(&ap);
    }

    #[test]
    fn group_size() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>::new();

        assert_ok_eq!(map.get_group_size(&ap), GroupReadResult::Uninitialized);

        map.set_group_base_values(
            ap.clone(),
            // base tag 1, 2, 3, 4
            (1..5).map(|i| (i, TestValue::creation_with_len(1))),
        );

        let tag: usize = 5;
        let tag_len = bcs::serialized_size(&tag).unwrap();
        let one_entry_len = TestValue::creation_with_len(1).bytes().unwrap().len();
        let two_entry_len = TestValue::creation_with_len(2).bytes().unwrap().len();
        let three_entry_len = TestValue::creation_with_len(3).bytes().unwrap().len();
        let four_entry_len = TestValue::creation_with_len(4).bytes().unwrap().len();

        let exp_size = 4 * one_entry_len + 4 * tag_len;
        assert_ok_eq!(
            map.get_group_size(&ap),
            GroupReadResult::Size(exp_size as u64)
        );

        assert_err!(map.insert_group_op(&ap, 0, TestValue::modification_with_len(2)));
        assert_ok!(map.insert_group_op(&ap, 0, TestValue::creation_with_len(2)));
        assert_err!(map.insert_group_op(&ap, 1, TestValue::creation_with_len(2)));
        assert_ok!(map.insert_group_op(&ap, 1, TestValue::modification_with_len(2)));
        let exp_size = 2 * two_entry_len + 3 * one_entry_len + 5 * tag_len;
        assert_ok_eq!(
            map.get_group_size(&ap),
            GroupReadResult::Size(exp_size as u64)
        );

        assert_ok!(map.insert_group_op(&ap, 4, TestValue::modification_with_len(3)));
        assert_ok!(map.insert_group_op(&ap, 5, TestValue::creation_with_len(3)));
        let exp_size = exp_size + 2 * three_entry_len + tag_len - one_entry_len;
        assert_ok_eq!(
            map.get_group_size(&ap),
            GroupReadResult::Size(exp_size as u64)
        );

        assert_ok!(map.insert_group_op(&ap, 0, TestValue::modification_with_len(4)));
        assert_ok!(map.insert_group_op(&ap, 1, TestValue::modification_with_len(4)));
        let exp_size = 2 * four_entry_len + 2 * three_entry_len + 2 * one_entry_len + 6 * tag_len;
        assert_ok_eq!(
            map.get_group_size(&ap),
            GroupReadResult::Size(exp_size as u64)
        );
    }

    #[test]
    fn group_value() {
        let ap = KeyType(b"/foo/f".to_vec());
        let map = UnsyncMap::<KeyType<Vec<u8>>, usize, TestValue, ExecutableTestType, ()>::new();

        assert_eq!(
            map.get_value_from_group(&ap, &1),
            GroupReadResult::Uninitialized
        );

        map.set_group_base_values(
            ap.clone(),
            // base tag 1, 2, 3, 4
            (1..5).map(|i| (i, TestValue::creation_with_len(i))),
        );

        for i in 1..5 {
            assert_eq!(
                map.get_value_from_group(&ap, &i),
                GroupReadResult::Value(TestValue::creation_with_len(i).bytes().cloned(), None)
            )
        }
        assert_eq!(
            map.get_value_from_group(&ap, &0),
            GroupReadResult::Value(None, None)
        );
        assert_eq!(
            map.get_value_from_group(&ap, &6),
            GroupReadResult::Value(None, None)
        );

        assert_ok!(map.insert_group_op(&ap, 1, TestValue::deletion()));
        assert_ok!(map.insert_group_op(&ap, 3, TestValue::modification_with_len(8)));
        assert_ok!(map.insert_group_op(&ap, 6, TestValue::creation_with_len(9)));

        assert_eq!(
            map.get_value_from_group(&ap, &1),
            GroupReadResult::Value(None, None)
        );
        assert_eq!(
            map.get_value_from_group(&ap, &3),
            GroupReadResult::Value(TestValue::creation_with_len(8).bytes().cloned(), None)
        );
        assert_eq!(
            map.get_value_from_group(&ap, &6),
            GroupReadResult::Value(TestValue::creation_with_len(9).bytes().cloned(), None)
        );

        // others unaffected.
        assert_eq!(
            map.get_value_from_group(&ap, &0),
            GroupReadResult::Value(None, None)
        );
        assert_eq!(
            map.get_value_from_group(&ap, &2),
            GroupReadResult::Value(TestValue::creation_with_len(2).bytes().cloned(), None)
        );
        assert_eq!(
            map.get_value_from_group(&ap, &4),
            GroupReadResult::Value(TestValue::creation_with_len(4).bytes().cloned(), None)
        );
    }
}
