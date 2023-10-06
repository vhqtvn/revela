// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::StorageAdapter,
    move_vm_ext::{AptosMoveResolver, SessionExt, SessionId},
    AptosVM,
};
use aptos_aggregator::resolver::{AggregatorReadMode, TAggregatorView};
use aptos_gas_algebra::Fee;
use aptos_state_view::StateViewId;
use aptos_types::{
    aggregator::AggregatorID,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::{
    change_set::VMChangeSet,
    resolver::{
        ExecutorView, ResourceGroupView, StateStorageView, TModuleView, TResourceGroupView,
        TResourceView,
    },
    storage::ChangeSetConfigs,
};
use bytes::Bytes;
use move_core_types::{
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::{err_msg, StatusCode, VMStatus},
};

/// We finish the session after the user transaction is done running to get the change set and
/// charge gas and storage fee based on it before running storage refunds and the transaction
/// epilogue. The latter needs to see the state view as if the change set is applied on top of
/// the base state view, and this struct implements that.
#[ouroboros::self_referencing]
pub struct RespawnedSession<'r, 'l> {
    executor_view: ExecutorViewWithChangeSet<'r>,
    #[borrows(executor_view)]
    #[covariant]
    resolver: StorageAdapter<'this, ExecutorViewWithChangeSet<'r>>,
    #[borrows(resolver)]
    #[not_covariant]
    session: Option<SessionExt<'this, 'l>>,
    pub storage_refund: Fee,
}

impl<'r, 'l> RespawnedSession<'r, 'l> {
    pub fn spawn(
        vm: &'l AptosVM,
        session_id: SessionId,
        base: &'r dyn AptosMoveResolver,
        previous_session_change_set: VMChangeSet,
        storage_refund: Fee,
    ) -> Result<Self, VMStatus> {
        let executor_view = ExecutorViewWithChangeSet::new(
            base.as_executor_view(),
            base.as_resource_group_view(),
            previous_session_change_set,
        );

        Ok(RespawnedSessionBuilder {
            executor_view,
            resolver_builder: |executor_view| vm.as_move_resolver(executor_view),
            session_builder: |resolver| Some(vm.0.new_session(resolver, session_id)),
            storage_refund,
        }
        .build())
    }

    pub fn execute<R>(&mut self, fun: impl FnOnce(&mut SessionExt) -> R) -> R {
        self.with_session_mut(|session| fun(session.as_mut().unwrap()))
    }

    pub fn finish(
        mut self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<VMChangeSet, VMStatus> {
        let additional_change_set = self.with_session_mut(|session| {
            session.take().unwrap().finish(&mut (), change_set_configs)
        })?;
        if additional_change_set.has_creation() {
            // After respawning, for example, in the epilogue, there shouldn't be new slots
            // created, otherwise there's a potential vulnerability like this:
            // 1. slot created by the user
            // 2. another user transaction deletes the slot and claims the refund
            // 3. in the epilogue the same slot gets recreated, and the final write set will have
            //    a ModifyWithMetadata carrying the original metadata
            // 4. user keeps doing the same and repeatedly claim refund out of the slot.
            return Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg("Unexpected storage allocation after respawning session."),
            ));
        }
        let mut change_set = self.into_heads().executor_view.change_set;
        change_set
            .squash_additional_change_set(additional_change_set, change_set_configs)
            .map_err(|_err| {
                VMStatus::error(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    err_msg("Failed to squash VMChangeSet"),
                )
            })?;
        Ok(change_set)
    }

    pub fn get_storage_fee_refund(&self) -> Fee {
        *self.borrow_storage_refund()
    }
}

/// Adapter to allow resolving the calls to `ExecutorView` via change set.
pub struct ExecutorViewWithChangeSet<'r> {
    base_executor_view: &'r dyn ExecutorView,
    base_resource_group_view: &'r dyn ResourceGroupView,
    change_set: VMChangeSet,
}

impl<'r> ExecutorViewWithChangeSet<'r> {
    pub(crate) fn new(
        base_executor_view: &'r dyn ExecutorView,
        base_resource_group_view: &'r dyn ResourceGroupView,
        change_set: VMChangeSet,
    ) -> Self {
        Self {
            base_executor_view,
            base_resource_group_view,
            change_set,
        }
    }
}

impl<'r> TAggregatorView for ExecutorViewWithChangeSet<'r> {
    type IdentifierV1 = StateKey;
    type IdentifierV2 = AggregatorID;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::IdentifierV1,
        mode: AggregatorReadMode,
    ) -> anyhow::Result<Option<StateValue>> {
        match self.change_set.aggregator_v1_delta_set().get(id) {
            Some(delta_op) => Ok(self
                .base_executor_view
                .try_convert_aggregator_v1_delta_into_write_op(id, delta_op, mode)?
                .as_state_value()),
            None => match self.change_set.aggregator_v1_write_set().get(id) {
                Some(write_op) => Ok(write_op.as_state_value()),
                None => self
                    .base_executor_view
                    .get_aggregator_v1_state_value(id, mode),
            },
        }
    }
}

impl<'r> TResourceView for ExecutorViewWithChangeSet<'r> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<StateValue>> {
        match self.change_set.resource_write_set().get(state_key) {
            Some(write_op) => Ok(write_op.as_state_value()),
            None => self
                .base_executor_view
                .get_resource_state_value(state_key, maybe_layout),
        }
    }
}

impl<'r> TResourceGroupView for ExecutorViewWithChangeSet<'r> {
    type GroupKey = StateKey;
    type Layout = MoveTypeLayout;
    type ResourceTag = StructTag;

    fn resource_group_size(&self, _group_key: &Self::GroupKey) -> anyhow::Result<u64> {
        // In respawned session, gas is irrelevant, so we return 0 (GroupSizeKind::None).
        Ok(0)
    }

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&Self::Layout>,
    ) -> anyhow::Result<Option<Bytes>> {
        if let Some(write_op) = self
            .change_set
            .resource_group_write_set()
            .get(group_key)
            .and_then(|g| g.inner_ops.get(resource_tag))
        {
            Ok(write_op.extract_raw_bytes())
        } else {
            self.base_resource_group_view.get_resource_from_group(
                group_key,
                resource_tag,
                maybe_layout,
            )
        }
    }
}

impl<'r> TModuleView for ExecutorViewWithChangeSet<'r> {
    type Key = StateKey;

    fn get_module_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        match self.change_set.module_write_set().get(state_key) {
            Some(write_op) => Ok(write_op.as_state_value()),
            None => self.base_executor_view.get_module_state_value(state_key),
        }
    }
}

impl<'r> StateStorageView for ExecutorViewWithChangeSet<'r> {
    fn id(&self) -> StateViewId {
        self.base_executor_view.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        anyhow::bail!("Unexpected access to get_usage()")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        data_cache::AsMoveResolver,
        move_vm_ext::resolver::{AsExecutorView, AsResourceGroupView},
    };
    use aptos_aggregator::delta_change_set::{delta_add, serialize};
    use aptos_language_e2e_tests::data_store::FakeDataStore;
    use aptos_types::{account_address::AccountAddress, write_set::WriteOp};
    use aptos_vm_types::{change_set::GroupWrite, check_change_set::CheckChangeSet};
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use std::collections::{BTreeMap, HashMap};

    /// A mock for testing. Always succeeds on checking a change set.
    struct NoOpChangeSetChecker;

    impl CheckChangeSet for NoOpChangeSetChecker {
        fn check_change_set(&self, _change_set: &VMChangeSet) -> anyhow::Result<(), VMStatus> {
            Ok(())
        }
    }

    fn key(s: impl ToString) -> StateKey {
        StateKey::raw(s.to_string().into_bytes())
    }

    fn write(v: u128) -> WriteOp {
        WriteOp::Modification(serialize(&v).into())
    }

    fn read_resource(view: &ExecutorViewWithChangeSet, s: impl ToString) -> u128 {
        bcs::from_bytes(&view.get_resource_bytes(&key(s), None).unwrap().unwrap()).unwrap()
    }

    fn read_module(view: &ExecutorViewWithChangeSet, s: impl ToString) -> u128 {
        bcs::from_bytes(&view.get_module_bytes(&key(s)).unwrap().unwrap()).unwrap()
    }

    fn read_aggregator(view: &ExecutorViewWithChangeSet, s: impl ToString) -> u128 {
        view.get_aggregator_v1_value(&key(s), AggregatorReadMode::Precise)
            .unwrap()
            .unwrap()
    }

    fn read_resource_from_group(
        view: &ExecutorViewWithChangeSet,
        s: impl ToString,
        tag: &StructTag,
    ) -> u128 {
        bcs::from_bytes(
            &view
                .get_resource_from_group(&key(s), tag, None)
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    fn mock_tag_0() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("a").unwrap(),
            name: Identifier::new("a").unwrap(),
            type_params: vec![TypeTag::U8],
        }
    }

    fn mock_tag_1() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcde").unwrap(),
            name: Identifier::new("fgh").unwrap(),
            type_params: vec![TypeTag::U64],
        }
    }

    fn mock_tag_2() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcdex").unwrap(),
            name: Identifier::new("fghx").unwrap(),
            type_params: vec![TypeTag::U128],
        }
    }

    #[test]
    fn test_change_set_state_view() {
        let mut state_view = FakeDataStore::default();
        state_view.set_legacy(key("module_base"), serialize(&10));
        state_view.set_legacy(key("module_both"), serialize(&20));

        state_view.set_legacy(key("resource_base"), serialize(&30));
        state_view.set_legacy(key("resource_both"), serialize(&40));

        state_view.set_legacy(key("aggregator_base"), serialize(&50));
        state_view.set_legacy(key("aggregator_both"), serialize(&60));
        state_view.set_legacy(key("aggregator_delta_set"), serialize(&70));

        let tree: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), serialize(&100).into()),
            (mock_tag_1(), serialize(&200).into()),
        ]);
        state_view.set_legacy(key("resource_group_base"), bcs::to_bytes(&tree).unwrap());
        state_view.set_legacy(key("resource_group_both"), bcs::to_bytes(&tree).unwrap());

        let resource_write_set = HashMap::from([
            (key("resource_both"), write(80)),
            (key("resource_write_set"), write(90)),
        ]);

        let module_write_set = HashMap::from([
            (key("module_both"), write(100)),
            (key("module_write_set"), write(110)),
        ]);

        let aggregator_write_set = HashMap::from([
            (key("aggregator_both"), write(120)),
            (key("aggregator_write_set"), write(130)),
        ]);

        let aggregator_delta_set =
            HashMap::from([(key("aggregator_delta_set"), delta_add(1, 1000))]);

        let resource_group_write_set = HashMap::from([
            (key("resource_group_both"), GroupWrite {
                metadata_op: WriteOp::Deletion, // should be irrelevant for the test
                inner_ops: HashMap::from([
                    (mock_tag_0(), WriteOp::Modification(serialize(&1000).into())),
                    (mock_tag_2(), WriteOp::Modification(serialize(&300).into())),
                ]),
            }),
            (key("resource_group_write_set"), GroupWrite {
                metadata_op: WriteOp::Deletion, // should be irrelevant for the test
                inner_ops: HashMap::from([(
                    mock_tag_1(),
                    WriteOp::Modification(serialize(&5000).into()),
                )]),
            }),
        ]);

        let change_set = VMChangeSet::new(
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set,
            vec![],
            &NoOpChangeSetChecker,
        )
        .unwrap();

        let resolver = state_view.as_move_resolver();
        let view = ExecutorViewWithChangeSet::new(
            resolver.as_executor_view(),
            resolver.as_resource_group_view(),
            change_set,
        );

        assert_eq!(read_module(&view, "module_base"), 10);
        assert_eq!(read_module(&view, "module_both"), 100);
        assert_eq!(read_module(&view, "module_write_set"), 110);

        assert_eq!(read_resource(&view, "resource_base"), 30);
        assert_eq!(read_resource(&view, "resource_both"), 80);
        assert_eq!(read_resource(&view, "resource_write_set"), 90);

        assert_eq!(read_aggregator(&view, "aggregator_base"), 50);
        assert_eq!(read_aggregator(&view, "aggregator_both"), 120);
        assert_eq!(read_aggregator(&view, "aggregator_write_set"), 130);
        assert_eq!(read_aggregator(&view, "aggregator_delta_set"), 71);

        assert_eq!(
            read_resource_from_group(&view, "resource_group_base", &mock_tag_0()),
            100
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_base", &mock_tag_1()),
            200
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_both", &mock_tag_0()),
            1000
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_both", &mock_tag_1()),
            200
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_both", &mock_tag_2()),
            300
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_write_set", &mock_tag_1()),
            5000
        );
    }
}
