// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::delta_change_set::{addition, deserialize, serialize, subtraction};
use aptos_types::vm_status::StatusCode;
use move_deps::{
    move_binary_format::errors::{PartialVMError, PartialVMResult},
    move_table_extension::{TableHandle, TableResolver},
};
use std::collections::{BTreeMap, BTreeSet};

/// Describes the state of each aggregator instance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AggregatorState {
    // If aggregator stores a known value.
    Data,
    // If aggregator stores a non-negative delta.
    PositiveDelta,
}

/// Uniquely identifies each aggregator instance in storage.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregatorID {
    // A handle that is shared accross all aggregator instances created by the
    // same `AggregatorFactory` and which is used for fine-grained storage
    // access.
    pub handle: u128,
    // Unique key associated with each aggregator instance. Generated by
    // taking the hash of transaction which creates an aggregator and the
    // number of aggregators that were created by this transaction so far.
    pub key: u128,
}

impl AggregatorID {
    pub fn new(handle: u128, key: u128) -> Self {
        AggregatorID { handle, key }
    }
}

/// Internal aggregator data structure.
pub struct Aggregator {
    // Describes a value of an aggregator.
    value: u128,
    // Describes a state of an aggregator.
    state: AggregatorState,
    // Describes an upper bound of an aggregator. If `value` exceeds it, the
    // aggregator overflows.
    // TODO: Currently this is a single u128 value since we use 0 as a trivial
    // lower bound. If we want to support custom lower bounds, or have more
    // complex postconditions, we should factor this out in its own struct.
    limit: u128,
}

impl Aggregator {
    /// Implements logic for adding to an aggregator.
    pub fn add(&mut self, value: u128) -> PartialVMResult<()> {
        // At this point, aggregator holds a positive delta or knows the value.
        // Hence, we can add, of course checking for overflow.
        self.value = addition(self.value, value, self.limit)?;
        Ok(())
    }

    /// Implements logic for subtracting from an aggregator.
    pub fn sub(&mut self, value: u128) -> PartialVMResult<()> {
        match self.state {
            AggregatorState::Data => {
                // Aggregator knows the value, therefore we can subtract
                // checking we don't drop below zero.
                self.value = subtraction(self.value, value)?;
                Ok(())
            }
            // For now, `aggregator::sub` always materializes the value, so
            // this should be unreachable.
            // TODO: support non-materialized subtractions.
            AggregatorState::PositiveDelta => {
                unreachable!("subtraction always materializes the value")
            }
        }
    }

    /// Implements logic for reading the value of an aggregator. As a
    /// result, the aggregator knows it value (i.e. its state changes to
    /// `Data`).
    pub fn read_and_materialize(
        &mut self,
        resolver: &dyn TableResolver,
        id: &AggregatorID,
    ) -> PartialVMResult<u128> {
        // If aggregator has already been read, return immediately.
        if self.state == AggregatorState::Data {
            return Ok(self.value);
        }

        // Otherwise, we have a delta and have to go to storage and apply it.
        // In theory, any delta will be applied to existing value. However,
        // something may go wrong, so we guard by throwing an error in
        // extension.
        let key_bytes = serialize(&id.key);
        resolver
            .resolve_table_entry(&TableHandle(id.handle), &key_bytes)
            .map_err(|_| extension_error("could not find the value of the aggregator"))?
            .map_or(
                Err(extension_error(
                    "could not find the value of the aggregator",
                )),
                |bytes| {
                    // The only remaining case is PositiveDelta. Assert just in
                    // case.
                    debug_assert!(self.state == AggregatorState::PositiveDelta);

                    // Get the value from the storage and try to apply the delta
                    // to it. If application succeeds, we change the state of the
                    // aggregator. Otherwise the error is propagated to the caller.
                    let base = deserialize(&bytes);
                    self.value = addition(base, self.value, self.limit)?;
                    self.state = AggregatorState::Data;

                    // Return the new value.
                    Ok(self.value)
                },
            )
    }

    /// Unpacks aggregator into its fields.
    pub fn into(self) -> (u128, AggregatorState, u128) {
        (self.value, self.state, self.limit)
    }
}

/// Stores all information about aggregators (how many have been created or
/// removed), what are their states, etc. per single transaction).
#[derive(Default)]
pub struct AggregatorData {
    // All aggregators that were created in the current transaction, stored as ids.
    // Used to filter out aggregators that were created and destroyed in the
    // within a single transaction.
    new_aggregators: BTreeSet<AggregatorID>,
    // All aggregators that were destroyed in the current transaction, stored as ids.
    destroyed_aggregators: BTreeSet<AggregatorID>,
    // All aggregator instances that exist in the current transaction.
    aggregators: BTreeMap<AggregatorID, Aggregator>,
}

impl AggregatorData {
    /// Returns a mutable reference to an aggregator with `id` and a `limit`.
    /// If transaction that is currently executing did not initilize it), a new
    /// aggregator instance is created, with a zero-initialized value and in a
    /// delta state.
    /// Note: when we say "aggregator instance" here we refer to Rust struct and
    /// not to the Move aggregator.
    pub fn get_aggregator(&mut self, id: AggregatorID, limit: u128) -> &mut Aggregator {
        self.aggregators.entry(id).or_insert_with(|| Aggregator {
            value: 0,
            state: AggregatorState::PositiveDelta,
            limit,
        });
        self.aggregators.get_mut(&id).unwrap()
    }

    /// Returns the number of aggregators that are used in the current transaction.
    pub fn num_aggregators(&self) -> u128 {
        self.aggregators.len() as u128
    }

    /// Creates and a new Aggregator with a given `id` and a `limit`. The value
    /// of a new aggregator is always known, therefore it is created in a data
    /// state, with a zero-initialized value.
    pub fn create_new_aggregator(&mut self, id: AggregatorID, limit: u128) {
        let aggregator = Aggregator {
            value: 0,
            state: AggregatorState::Data,
            limit,
        };
        self.aggregators.insert(id, aggregator);
        self.new_aggregators.insert(id);
    }

    /// If aggregator has been used in this transaction, it is removed. Otherwise,
    /// it is marked for deletion.
    pub fn remove_aggregator(&mut self, id: AggregatorID) {
        // Aggregator no longer in use during this transaction: remove it.
        self.aggregators.remove(&id);

        if self.new_aggregators.contains(&id) {
            // Aggregator has been created in the same transaction. Therefore, no
            // side-effects.
            self.new_aggregators.remove(&id);
        } else {
            // Otherwise, aggregator has been created somewhere else.
            self.destroyed_aggregators.insert(id);
        }
    }

    /// Unpacks aggregator data.
    pub fn into(
        self,
    ) -> (
        BTreeSet<AggregatorID>,
        BTreeSet<AggregatorID>,
        BTreeMap<AggregatorID, Aggregator>,
    ) {
        (
            self.new_aggregators,
            self.destroyed_aggregators,
            self.aggregators,
        )
    }
}

/// Returns partial VM error on extension failure.
pub fn extension_error(message: impl ToString) -> PartialVMError {
    PartialVMError::new(StatusCode::VM_EXTENSION_ERROR).with_message(message.to_string())
}

// ================================= Tests =================================

#[cfg(test)]
mod test {
    use super::*;
    use aptos_state_view::state_storage_usage::StateStorageUsage;
    use aptos_state_view::StateView;
    use aptos_types::state_store::{state_key::StateKey, table::TableHandle as AptosTableHandle};
    use claim::{assert_err, assert_matches, assert_ok};
    use move_deps::{
        move_core_types::gas_algebra::InternalGas, move_table_extension::TableOperation,
    };
    use once_cell::sync::Lazy;
    use std::collections::HashMap;

    #[derive(Default)]
    pub struct FakeTestStorage {
        data: HashMap<StateKey, Vec<u8>>,
    }

    impl FakeTestStorage {
        fn new() -> Self {
            let mut data = HashMap::new();

            // Initialize storage with some test data.
            data.insert(id_to_state_key(test_id(4)), serialize(&900));
            data.insert(id_to_state_key(test_id(5)), serialize(&5));
            FakeTestStorage { data }
        }
    }

    impl StateView for FakeTestStorage {
        fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(self.data.get(state_key).cloned())
        }

        fn is_genesis(&self) -> bool {
            self.data.is_empty()
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            Ok(StateStorageUsage::new_untracked())
        }
    }

    impl TableResolver for FakeTestStorage {
        fn resolve_table_entry(
            &self,
            handle: &TableHandle,
            key: &[u8],
        ) -> Result<Option<Vec<u8>>, anyhow::Error> {
            let state_key = StateKey::table_item(AptosTableHandle::from(*handle), key.to_vec());
            self.get_state_value(&state_key)
        }

        fn operation_cost(
            &self,
            _op: TableOperation,
            _key_size: usize,
            _val_size: usize,
        ) -> InternalGas {
            1.into()
        }
    }

    fn test_id(key: u128) -> AggregatorID {
        AggregatorID::new(0, key)
    }

    fn id_to_state_key(id: AggregatorID) -> StateKey {
        let key_bytes = serialize(&id.key);
        StateKey::table_item(AptosTableHandle(id.handle), key_bytes)
    }

    #[allow(clippy::redundant_closure)]
    static TEST_RESOLVER: Lazy<FakeTestStorage> = Lazy::new(|| FakeTestStorage::new());

    fn test_set_up(aggregator_data: &mut AggregatorData) {
        // Aggregators with data.
        aggregator_data.create_new_aggregator(test_id(1), 1000);

        // Aggregators with delta.
        aggregator_data.get_aggregator(test_id(4), 1000);
        aggregator_data.get_aggregator(test_id(5), 10);
    }

    #[test]
    fn test_aggregator_operations() {
        let mut aggregator_data = AggregatorData::default();
        test_set_up(&mut aggregator_data);

        // This aggregator has been created by this transaction, hence the
        // value is known.
        let aggregator = aggregator_data.get_aggregator(test_id(1), 1000);
        assert_matches!(aggregator.state, AggregatorState::Data);
        assert_eq!(aggregator.value, 0);

        assert_ok!(aggregator.add(100));
        assert_ok!(aggregator.add(900));
        assert_matches!(aggregator.state, AggregatorState::Data);
        assert_eq!(aggregator.value, 1000);

        // Overflow!
        assert_err!(aggregator.add(1));

        // This aggregator has not been created by this transaction, and contains
        // an unknown value.
        let aggregator = aggregator_data.get_aggregator(test_id(4), 1000);
        assert_matches!(aggregator.state, AggregatorState::PositiveDelta);
        assert_eq!(aggregator.value, 0);

        assert_ok!(aggregator.add(100));
        assert_ok!(aggregator.add(100));
        assert_matches!(aggregator.state, AggregatorState::PositiveDelta);
        assert_eq!(aggregator.value, 200);

        // 900 + 200 > 1000!
        assert_err!(aggregator.read_and_materialize(&*TEST_RESOLVER, &test_id(4)));

        // This aggregator also has not been created by this transaction, and
        // contains an unknown value.
        let aggregator = aggregator_data.get_aggregator(test_id(5), 10);
        assert_matches!(aggregator.state, AggregatorState::PositiveDelta);
        assert_eq!(aggregator.value, 0);

        assert_ok!(aggregator.add(2));
        assert_matches!(aggregator.state, AggregatorState::PositiveDelta);
        assert_eq!(aggregator.value, 2);

        assert_ok!(aggregator.read_and_materialize(&*TEST_RESOLVER, &test_id(5)));
        assert_matches!(aggregator.state, AggregatorState::Data);
        assert_eq!(aggregator.value, 7);

        assert_ok!(aggregator.sub(7));
        assert_matches!(aggregator.state, AggregatorState::Data);
        assert_eq!(aggregator.value, 0);
    }
}
