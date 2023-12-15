// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{session::BytesWithResourceLayout, AptosMoveResolver};
use aptos_aggregator::delta_change_set::serialize;
use aptos_types::{
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::WriteOp,
};
use aptos_vm_types::{
    abstract_write_op::GroupWrite, resolver::ResourceGroupSize,
    resource_group_adapter::group_tagged_resource_size,
};
use bytes::Bytes;
use move_core_types::{
    effects::Op as MoveStorageOp,
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::{err_msg, StatusCode, VMStatus},
};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) struct WriteOpConverter<'r> {
    remote: &'r dyn AptosMoveResolver,
    new_slot_metadata: Option<StateValueMetadata>,
}

macro_rules! convert_impl {
    ($convert_func_name:ident, $get_metadata_callback:ident) => {
        pub(crate) fn $convert_func_name(
            &self,
            state_key: &StateKey,
            move_storage_op: MoveStorageOp<Bytes>,
            legacy_creation_as_modification: bool,
        ) -> Result<WriteOp, VMStatus> {
            let move_storage_op = match move_storage_op {
                MoveStorageOp::New(data) => MoveStorageOp::New((data, None)),
                MoveStorageOp::Modify(data) => MoveStorageOp::Modify((data, None)),
                MoveStorageOp::Delete => MoveStorageOp::Delete,
            };
            self.convert(
                self.remote.$get_metadata_callback(state_key),
                move_storage_op,
                legacy_creation_as_modification,
            )
        }
    };
}

// We set SPECULATIVE_EXECUTION_ABORT_ERROR here, as the error can happen due to
// speculative reads (and in a non-speculative context, e.g. during commit, it
// is a more serious error and block execution must abort).
// BlockExecutor is responsible with handling this error.
fn group_size_arithmetics_error() -> VMStatus {
    VMStatus::error(
        StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
        err_msg("Group size arithmetics error while applying updates"),
    )
}

fn decrement_size_for_remove_tag(
    size: &mut ResourceGroupSize,
    old_tagged_resource_size: u64,
) -> Result<(), VMStatus> {
    match size {
        ResourceGroupSize::Concrete(_) => Err(VMStatus::error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            err_msg("Unexpected ResourceGroupSize::Concrete in convert_resource_group_v1"),
        )),
        ResourceGroupSize::Combined {
            num_tagged_resources,
            all_tagged_resources_size,
        } => {
            *num_tagged_resources = num_tagged_resources
                .checked_sub(1)
                .ok_or_else(group_size_arithmetics_error)?;
            *all_tagged_resources_size = all_tagged_resources_size
                .checked_sub(old_tagged_resource_size)
                .ok_or_else(group_size_arithmetics_error)?;
            Ok(())
        },
    }
}

fn increment_size_for_add_tag(
    size: &mut ResourceGroupSize,
    new_tagged_resource_size: u64,
) -> Result<(), VMStatus> {
    match size {
        ResourceGroupSize::Concrete(_) => Err(VMStatus::error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            err_msg("Unexpected ResourceGroupSize::Concrete in convert_resource_group_v1"),
        )),
        ResourceGroupSize::Combined {
            num_tagged_resources,
            all_tagged_resources_size,
        } => {
            *num_tagged_resources = num_tagged_resources
                .checked_add(1)
                .ok_or_else(group_size_arithmetics_error)?;
            *all_tagged_resources_size = all_tagged_resources_size
                .checked_add(new_tagged_resource_size)
                .ok_or_else(group_size_arithmetics_error)?;
            Ok(())
        },
    }
}

fn check_size_and_existance_match(
    size: &ResourceGroupSize,
    exists: bool,
    state_key: &StateKey,
) -> Result<(), VMStatus> {
    if exists {
        if size.get() == 0 {
            Err(VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(format!(
                    "Group tag count/size shouldn't be 0 for an existing group: {:?}",
                    state_key
                )),
            ))
        } else {
            Ok(())
        }
    } else if size.get() > 0 {
        Err(VMStatus::error(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
            err_msg(format!(
                "Group tag count/size should be 0 for a new group: {:?}",
                state_key
            )),
        ))
    } else {
        Ok(())
    }
}

impl<'r> WriteOpConverter<'r> {
    convert_impl!(convert_module, get_module_state_value_metadata);

    convert_impl!(convert_aggregator, get_aggregator_v1_state_value_metadata);

    pub(crate) fn new(
        remote: &'r dyn AptosMoveResolver,
        is_storage_slot_metadata_enabled: bool,
    ) -> Self {
        let mut new_slot_metadata: Option<StateValueMetadata> = None;
        if is_storage_slot_metadata_enabled {
            if let Some(current_time) = CurrentTimeMicroseconds::fetch_config(remote) {
                // The deposit on the metadata is a placeholder (0), it will be updated later when
                // storage fee is charged.
                new_slot_metadata = Some(StateValueMetadata::placeholder(&current_time));
            }
        }

        Self {
            remote,
            new_slot_metadata,
        }
    }

    pub(crate) fn convert_resource(
        &self,
        state_key: &StateKey,
        move_storage_op: MoveStorageOp<BytesWithResourceLayout>,
        legacy_creation_as_modification: bool,
    ) -> Result<(WriteOp, Option<Arc<MoveTypeLayout>>), VMStatus> {
        let result = self.convert(
            self.remote.get_resource_state_value_metadata(state_key),
            move_storage_op.clone(),
            legacy_creation_as_modification,
        );
        match move_storage_op {
            MoveStorageOp::New((_, type_layout)) => Ok((result?, type_layout)),
            MoveStorageOp::Modify((_, type_layout)) => Ok((result?, type_layout)),
            MoveStorageOp::Delete => Ok((result?, None)),
        }
    }

    pub(crate) fn convert_resource_group_v1(
        &self,
        state_key: &StateKey,
        group_changes: BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
    ) -> Result<GroupWrite, VMStatus> {
        // Resource group metadata is stored at the group StateKey, and can be obtained via the
        // same interfaces at for a resource at a given StateKey.
        let state_value_metadata_result = self.remote.get_resource_state_value_metadata(state_key);
        // Currently, due to read-before-write and a gas charge on the first read that is based
        // on the group size, this should simply re-read a cached (speculative) group size.
        let pre_group_size = self.remote.resource_group_size(state_key).map_err(|_| {
            VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg("Error querying resource group size"),
            )
        })?;

        if let Ok(v) = &state_value_metadata_result {
            check_size_and_existance_match(&pre_group_size, v.is_some(), state_key)?;
        }

        let mut inner_ops = BTreeMap::new();

        let tag_serialization_error = |_| {
            VMStatus::error(
                StatusCode::VALUE_SERIALIZATION_ERROR,
                err_msg("Tag serialization error"),
            )
        };

        let mut post_group_size = pre_group_size;

        for (tag, current_op) in group_changes {
            // We take speculative group size prior to the transaction, and update it based on the change-set.
            // For each tagged resource in the change set, we subtract the previous size tagged resource size,
            // and then add new tagged resource size.
            //
            // The reason we do not insteat get and add the sizes of the resources in the group,
            // but not in the change-set, is to avoid creating unnecessary R/W conflicts (the resources
            // in the change-set are already read, but the other resources are not).
            if !matches!(current_op, MoveStorageOp::New(_)) {
                let old_tagged_value_size = self
                    .remote
                    .resource_size_in_group(state_key, &tag)
                    .map_err(|_| {
                        VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Error querying resource group size"),
                        )
                    })?;
                let old_size = group_tagged_resource_size(&tag, old_tagged_value_size)
                    .map_err(tag_serialization_error)?;
                decrement_size_for_remove_tag(&mut post_group_size, old_size)?;
            }

            match &current_op {
                MoveStorageOp::Modify((data, _)) | MoveStorageOp::New((data, _)) => {
                    let new_size = group_tagged_resource_size(&tag, data.len())
                        .map_err(tag_serialization_error)?;
                    increment_size_for_add_tag(&mut post_group_size, new_size)?;
                },
                MoveStorageOp::Delete => {},
            };

            let legacy_op = match current_op {
                MoveStorageOp::Delete => (WriteOp::legacy_deletion(), None),
                MoveStorageOp::Modify((data, maybe_layout)) => {
                    (WriteOp::legacy_modification(data), maybe_layout)
                },
                MoveStorageOp::New((data, maybe_layout)) => {
                    (WriteOp::legacy_creation(data), maybe_layout)
                },
            };
            inner_ops.insert(tag, legacy_op);
        }

        // Create the op that would look like a combined V0 resource group MoveStorageOp,
        // except it encodes the (speculative) size of the group after applying the updates
        // which is used for charging storage fees. Moreover, the metadata computation occurs
        // fully backwards compatibly, and lets obtain final storage op by replacing bytes.
        // TODO[agg_v2](fix) fix layout for RG
        let metadata_op = if post_group_size.get() == 0 {
            MoveStorageOp::Delete
        } else if pre_group_size.get() == 0 {
            MoveStorageOp::New((Bytes::new(), None))
        } else {
            MoveStorageOp::Modify((Bytes::new(), None))
        };
        Ok(GroupWrite::new(
            self.convert(state_value_metadata_result, metadata_op, false)?,
            // TODO[agg_v2](fix): Converting the inner ops from Vec to BTreeMap. Try to have
            // uniform datastructure to represent the inner ops.
            inner_ops.into_iter().collect(),
            post_group_size.get(),
        ))
    }

    fn convert(
        &self,
        state_value_metadata_result: anyhow::Result<Option<StateValueMetadata>>,
        move_storage_op: MoveStorageOp<BytesWithResourceLayout>,
        legacy_creation_as_modification: bool,
    ) -> Result<WriteOp, VMStatus> {
        use MoveStorageOp::*;
        use WriteOp::*;

        let maybe_existing_metadata = state_value_metadata_result.map_err(|_| {
            VMStatus::error(
                StatusCode::STORAGE_ERROR,
                err_msg("Storage read failed when converting change set."),
            )
        })?;

        let write_op = match (maybe_existing_metadata, move_storage_op) {
            (None, Modify(_) | Delete) => {
                return Err(VMStatus::error(
                    // Possible under speculative execution, returning speculative error waiting for re-execution
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    err_msg("When converting write op: updating non-existent value."),
                ));
            },
            (Some(_), New(_)) => {
                return Err(VMStatus::error(
                    // Possible under speculative execution, returning speculative error waiting for re-execution
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    err_msg("When converting write op: Recreating existing value."),
                ));
            },
            (None, New((data, _))) => match &self.new_slot_metadata {
                None => {
                    if legacy_creation_as_modification {
                        WriteOp::legacy_modification(data)
                    } else {
                        WriteOp::legacy_creation(data)
                    }
                },
                Some(metadata) => Creation {
                    data,
                    metadata: metadata.clone(),
                },
            },
            (Some(existing_metadata), Modify((data, _))) => {
                // Inherit metadata even if the feature flags is turned off, for compatibility.
                Modification {
                    data,
                    metadata: existing_metadata,
                }
            },
            (Some(existing_metadata), Delete) => {
                // Inherit metadata even if the feature flags is turned off, for compatibility.
                Deletion {
                    metadata: existing_metadata,
                }
            },
        };
        Ok(write_op)
    }

    pub(crate) fn convert_aggregator_modification(
        &self,
        state_key: &StateKey,
        value: u128,
    ) -> Result<WriteOp, VMStatus> {
        let maybe_existing_metadata = self
            .remote
            .get_aggregator_v1_state_value_metadata(state_key)
            .map_err(|e| {
                VMStatus::error(
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                    Some(format!("convert_aggregator_modification failed {:?}", e).to_string()),
                )
            })?;
        let data = serialize(&value).into();

        let op = match maybe_existing_metadata {
            None => {
                match &self.new_slot_metadata {
                    // n.b. Aggregator writes historically did not distinguish Create vs Modify.
                    None => WriteOp::legacy_modification(data),
                    Some(metadata) => WriteOp::Creation {
                        data,
                        metadata: metadata.clone(),
                    },
                }
            },
            Some(metadata) => WriteOp::Modification { data, metadata },
        };

        Ok(op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_cache::tests::as_resolver_with_group_size_kind,
        move_vm_ext::resolver::ResourceGroupResolver,
    };
    use aptos_state_view::TStateView;
    use aptos_types::{
        account_address::AccountAddress,
        state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    };
    use aptos_vm_types::resource_group_adapter::{group_size_as_sum, GroupSizeKind};
    use claims::{assert_none, assert_some_eq};
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };

    fn raw_metadata(v: u64) -> StateValueMetadata {
        StateValueMetadata::legacy(v, &CurrentTimeMicroseconds { microseconds: v })
    }

    // TODO: Can re-use some of these testing definitions with aptos-vm-types.
    pub(crate) fn mock_tag_0() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("a").unwrap(),
            name: Identifier::new("a").unwrap(),
            type_params: vec![TypeTag::U8],
        }
    }

    pub(crate) fn mock_tag_1() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcde").unwrap(),
            name: Identifier::new("fgh").unwrap(),
            type_params: vec![TypeTag::U64],
        }
    }

    pub(crate) fn mock_tag_2() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcdex").unwrap(),
            name: Identifier::new("fghx").unwrap(),
            type_params: vec![TypeTag::U128],
        }
    }

    struct MockStateView {
        data: BTreeMap<StateKey, StateValue>,
    }

    impl MockStateView {
        fn new(data: BTreeMap<StateKey, StateValue>) -> Self {
            Self { data }
        }
    }

    impl TStateView for MockStateView {
        type Key = StateKey;

        fn get_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
            Ok(self.data.get(state_key).cloned())
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            unimplemented!();
        }
    }

    // TODO[agg_v2](fix) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_delete_modify_ops() {
        let group: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), vec![1].into()),
            (mock_tag_1(), vec![2, 2].into()),
            (mock_tag_2(), vec![3, 3, 3].into()),
        ]);
        let metadata = raw_metadata(100);
        let key = StateKey::raw(vec![0]);

        let data = BTreeMap::from([(
            key.clone(),
            StateValue::new_with_metadata(bcs::to_bytes(&group).unwrap().into(), metadata.clone()),
        )]);

        let expected_size = group_size_as_sum(
            vec![(&mock_tag_0(), 1), (&mock_tag_1(), 2), (&mock_tag_2(), 3)].into_iter(),
        )
        .unwrap();

        let s = MockStateView::new(data);
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);

        assert_eq!(resolver.resource_group_size(&key).unwrap(), expected_size);
        // TODO: Layout hardcoded to None. Test with layout = Some(..)
        let group_changes = BTreeMap::from([
            (mock_tag_0(), MoveStorageOp::Delete),
            (
                mock_tag_2(),
                MoveStorageOp::Modify((vec![5, 5, 5, 5, 5].into(), None)),
            ),
        ]);
        let converter = WriteOpConverter::new(&resolver, false);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        assert_eq!(group_write.metadata_op().metadata(), &metadata);
        let expected_new_size = bcs::serialized_size(&mock_tag_1()).unwrap()
            + bcs::serialized_size(&mock_tag_2()).unwrap()
            + 7; // values bytes size: 2 + 5
        assert_some_eq!(group_write.maybe_group_op_size(), expected_new_size as u64);
        assert_eq!(group_write.inner_ops().len(), 2);
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_0()),
            &(WriteOp::legacy_deletion(), None)
        );
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_2()),
            &(
                WriteOp::legacy_modification(vec![5, 5, 5, 5, 5].into()),
                None
            )
        );
    }

    // TODO[agg_v2](fix) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_new_op() {
        let group: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), vec![1].into()),
            (mock_tag_1(), vec![2, 2].into()),
        ]);
        let metadata = raw_metadata(100);
        let key = StateKey::raw(vec![0]);

        let data = BTreeMap::from([(
            key.clone(),
            StateValue::new_with_metadata(bcs::to_bytes(&group).unwrap().into(), metadata.clone()),
        )]);

        let s = MockStateView::new(data);
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);

        let group_changes = BTreeMap::from([(
            mock_tag_2(),
            MoveStorageOp::New((vec![3, 3, 3].into(), None)),
        )]);
        let converter = WriteOpConverter::new(&resolver, true);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        assert_eq!(group_write.metadata_op().metadata(), &metadata);
        let expected_new_size = bcs::serialized_size(&mock_tag_0()).unwrap()
            + bcs::serialized_size(&mock_tag_1()).unwrap()
            + bcs::serialized_size(&mock_tag_2()).unwrap()
            + 6; // values bytes size: 1 + 2 + 3.
        assert_some_eq!(group_write.maybe_group_op_size(), expected_new_size as u64);
        assert_eq!(group_write.inner_ops().len(), 1);
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_2()),
            &(WriteOp::legacy_creation(vec![3, 3, 3].into()), None)
        );
    }

    // TODO[agg_v2](fix) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_new_group() {
        let s = MockStateView::new(BTreeMap::new());
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);

        // TODO: Layout hardcoded to None. Test with layout = Some(..)
        let group_changes =
            BTreeMap::from([(mock_tag_1(), MoveStorageOp::New((vec![2, 2].into(), None)))]);
        let key = StateKey::raw(vec![0]);
        let converter = WriteOpConverter::new(&resolver, true);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        assert!(group_write.metadata_op().metadata().is_none());
        let expected_new_size = bcs::serialized_size(&mock_tag_1()).unwrap() + 2;
        assert_some_eq!(group_write.maybe_group_op_size(), expected_new_size as u64);
        assert_eq!(group_write.inner_ops().len(), 1);
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_1()),
            &(WriteOp::legacy_creation(vec![2, 2].into()), None)
        );
    }

    // TODO[agg_v2](fix) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_delete_group() {
        let group: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), vec![1].into()),
            (mock_tag_1(), vec![2, 2].into()),
        ]);
        let metadata = raw_metadata(100);
        let key = StateKey::raw(vec![0]);

        let data = BTreeMap::from([(
            key.clone(),
            StateValue::new_with_metadata(bcs::to_bytes(&group).unwrap().into(), metadata.clone()),
        )]);

        let s = MockStateView::new(data);
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);
        let group_changes = BTreeMap::from([
            (mock_tag_0(), MoveStorageOp::Delete),
            (mock_tag_1(), MoveStorageOp::Delete),
        ]);
        let converter = WriteOpConverter::new(&resolver, true);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        // Deletion should still contain the metadata - for storage refunds.
        assert_eq!(group_write.metadata_op().metadata(), &metadata);
        assert_eq!(group_write.metadata_op(), &WriteOp::Deletion { metadata });
        assert_none!(group_write.metadata_op().bytes());
    }
}
