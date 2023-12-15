// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
//! Scratchpad for on chain values during the execution.

use crate::{
    gas::get_gas_config_from_storage,
    move_vm_ext::{
        get_max_binary_format_version, get_max_identifier_size, AptosMoveResolver, AsExecutorView,
        AsResourceGroupView, ResourceGroupResolver,
    },
};
#[allow(unused_imports)]
use anyhow::{bail, Error};
use aptos_aggregator::{
    bounded_math::SignedU128,
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::{DelayedFieldID, DelayedFieldValue, DelayedFieldsSpeculativeError, PanicOr},
};
use aptos_state_view::{StateView, StateViewId};
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::{
    access_path::AccessPath,
    aggregator::PanicError,
    on_chain_config::{ConfigStorage, Features, OnChainConfig},
    state_store::{
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
    },
    write_set::WriteOp,
};
use aptos_vm_types::{
    resolver::{
        ExecutorView, ResourceGroupSize, ResourceGroupView, StateStorageView,
        StateValueMetadataResolver, TResourceGroupView, TResourceView,
    },
    resource_group_adapter::ResourceGroupAdapter,
};
use bytes::Bytes;
use move_binary_format::{deserializer::DeserializerConfig, errors::*, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{resource_size, ModuleResolver, ResourceResolver},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

pub(crate) fn get_resource_group_from_metadata(
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Option<StructTag> {
    let metadata = aptos_framework::get_metadata(metadata)?;
    metadata
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

struct ConfigAdapter<'a, K, L>(&'a dyn TResourceView<Key = K, Layout = L>);

impl<'a> ConfigStorage for ConfigAdapter<'a, StateKey, MoveTypeLayout> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Bytes> {
        self.0
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .ok()?
    }
}

/// Adapter to convert a `ExecutorView` into a `AptosMoveResolver`.
///
/// Resources in groups are handled either through dedicated interfaces of executor_view
/// (that tie to specialized handling in block executor), or via 'standard' interfaces
/// for (non-group) resources and subsequent handling in the StorageAdapter itself.
pub struct StorageAdapter<'e, E> {
    executor_view: &'e E,
    deserializer_config: DeserializerConfig,
    resource_group_view: ResourceGroupAdapter<'e>,
    accessed_groups: RefCell<HashSet<StateKey>>,
}

impl<'e, E: ExecutorView> StorageAdapter<'e, E> {
    pub(crate) fn new_with_config(
        executor_view: &'e E,
        gas_feature_version: u64,
        features: &Features,
        maybe_resource_group_view: Option<&'e dyn ResourceGroupView>,
    ) -> Self {
        let max_binary_version = get_max_binary_format_version(features, Some(gas_feature_version));
        let max_identifier_size = get_max_identifier_size(features);
        let resource_group_adapter = ResourceGroupAdapter::new(
            maybe_resource_group_view,
            executor_view,
            gas_feature_version,
            features.is_resource_group_charge_as_size_sum_enabled(),
        );

        Self::new(
            executor_view,
            max_binary_version,
            max_identifier_size,
            resource_group_adapter,
        )
    }

    fn new(
        executor_view: &'e E,
        max_binary_format_version: u32,
        max_identifier_size: u64,
        resource_group_view: ResourceGroupAdapter<'e>,
    ) -> Self {
        Self {
            executor_view,
            deserializer_config: DeserializerConfig::new(
                max_binary_format_version,
                max_identifier_size,
            ),
            resource_group_view,
            accessed_groups: RefCell::new(HashSet::new()),
        }
    }

    fn get_any_resource_with_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        // Question: Is maybe_layout = Some(..) iff the layout has an aggregator v2
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<(Option<Bytes>, usize), VMError> {
        let resource_group = get_resource_group_from_metadata(struct_tag, metadata);
        if let Some(resource_group) = resource_group {
            // TODO[agg_v2](fix) pass the layout to resource groups

            let key = StateKey::access_path(AccessPath::resource_group_access_path(
                *address,
                resource_group.clone(),
            ));

            let first_access = self.accessed_groups.borrow_mut().insert(key.clone());
            let common_error = |e| -> VMError {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(format!("{}", e))
                    .finish(Location::Undefined)
            };

            let buf = self
                .resource_group_view
                .get_resource_from_group(&key, struct_tag, maybe_layout)
                .map_err(common_error)?;
            let group_size = if first_access {
                self.resource_group_view
                    .resource_group_size(&key)
                    .map_err(common_error)?
                    .get()
            } else {
                0
            };

            let buf_size = resource_size(&buf);
            Ok((buf, buf_size + group_size as usize))
        } else {
            let access_path = AccessPath::resource_access_path(*address, struct_tag.clone())
                .map_err(|_| {
                    PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Undefined)
                })?;

            let buf = self
                .executor_view
                .get_resource_bytes(&StateKey::access_path(access_path), maybe_layout)
                .map_err(|_| {
                    PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
                })?;
            let buf_size = resource_size(&buf);
            Ok((buf, buf_size))
        }
    }
}

impl<'e, E: ExecutorView> ResourceGroupResolver for StorageAdapter<'e, E> {
    fn release_resource_group_cache(
        &self,
    ) -> Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>> {
        self.resource_group_view.release_group_cache()
    }

    fn resource_group_size(&self, group_key: &StateKey) -> anyhow::Result<ResourceGroupSize> {
        self.resource_group_view.resource_group_size(group_key)
    }

    fn resource_size_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> anyhow::Result<usize> {
        self.resource_group_view
            .resource_size_in_group(group_key, resource_tag)
    }

    fn resource_exists_in_group(
        &self,
        group_key: &StateKey,
        resource_tag: &StructTag,
    ) -> anyhow::Result<bool> {
        self.resource_group_view
            .resource_exists_in_group(group_key, resource_tag)
    }
}

impl<'e, E: ExecutorView> AptosMoveResolver for StorageAdapter<'e, E> {}

impl<'e, E: ExecutorView> ResourceResolver for StorageAdapter<'e, E> {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> anyhow::Result<(Option<Bytes>, usize)> {
        Ok(self.get_any_resource_with_layout(address, struct_tag, metadata, maybe_layout)?)
    }
}

impl<'e, E: ExecutorView> ModuleResolver for StorageAdapter<'e, E> {
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
        let module_bytes = match self.get_module(module_id) {
            Ok(Some(bytes)) => bytes,
            _ => return vec![],
        };
        let module =
            match CompiledModule::deserialize_with_config(&module_bytes, &self.deserializer_config)
            {
                Ok(module) => module,
                _ => return vec![],
            };
        module.metadata
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Bytes>, Error> {
        let access_path = AccessPath::from(module_id);
        Ok(self
            .executor_view
            .get_module_bytes(&StateKey::access_path(access_path))
            .map_err(|_| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).finish(Location::Undefined)
            })?)
    }
}

impl<'e, E: ExecutorView> TableResolver for StorageAdapter<'e, E> {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Bytes>, Error> {
        self.executor_view.get_resource_bytes(
            &StateKey::table_item((*handle).into(), key.to_vec()),
            layout,
        )
    }
}

impl<'e, E: ExecutorView> TAggregatorV1View for StorageAdapter<'e, E> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> anyhow::Result<Option<StateValue>> {
        self.executor_view.get_aggregator_v1_state_value(id)
    }
}

impl<'e, E: ExecutorView> TDelayedFieldView for StorageAdapter<'e, E> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;
    type ResourceValue = WriteOp;

    fn is_delayed_field_optimization_capable(&self) -> bool {
        self.executor_view.is_delayed_field_optimization_capable()
    }

    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        self.executor_view.get_delayed_field_value(id)
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        self.executor_view
            .delayed_field_try_add_delta_outcome(id, base_delta, delta, max_value)
    }

    fn generate_delayed_field_id(&self) -> Self::Identifier {
        self.executor_view.generate_delayed_field_id()
    }

    fn validate_and_convert_delayed_field_id(
        &self,
        id: u64,
    ) -> Result<Self::Identifier, PanicError> {
        self.executor_view.validate_and_convert_delayed_field_id(id)
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<BTreeMap<Self::ResourceKey, (Self::ResourceValue, Arc<MoveTypeLayout>)>, PanicError>
    {
        self.executor_view
            .get_reads_needing_exchange(delayed_write_set_keys, skip)
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<BTreeMap<Self::ResourceKey, (Self::ResourceValue, u64)>, PanicError> {
        self.executor_view
            .get_group_reads_needing_exchange(delayed_write_set_keys, skip)
    }
}

impl<'e, E: ExecutorView> ConfigStorage for StorageAdapter<'e, E> {
    fn fetch_config(&self, access_path: AccessPath) -> Option<Bytes> {
        self.executor_view
            .get_resource_bytes(&StateKey::access_path(access_path), None)
            .ok()?
    }
}

/// Converts `StateView` into `AptosMoveResolver`.
pub trait AsMoveResolver<S> {
    fn as_move_resolver(&self) -> StorageAdapter<S>;
}

impl<S: StateView> AsMoveResolver<S> for S {
    fn as_move_resolver(&self) -> StorageAdapter<S> {
        let config_view = ConfigAdapter(self);
        let (_, gas_feature_version) = get_gas_config_from_storage(&config_view);
        let features = Features::fetch_config(&config_view).unwrap_or_default();
        let max_binary_version =
            get_max_binary_format_version(&features, Some(gas_feature_version));
        let resource_group_adapter = ResourceGroupAdapter::new(
            None,
            self,
            gas_feature_version,
            features.is_resource_group_charge_as_size_sum_enabled(),
        );
        let max_identifier_size = get_max_identifier_size(&features);
        StorageAdapter::new(
            self,
            max_binary_version,
            max_identifier_size,
            resource_group_adapter,
        )
    }
}

impl<'e, E: ExecutorView> StateStorageView for StorageAdapter<'e, E> {
    fn id(&self) -> StateViewId {
        self.executor_view.id()
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        self.executor_view.get_usage()
    }
}

impl<'e, E: ExecutorView> StateValueMetadataResolver for StorageAdapter<'e, E> {
    fn get_module_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadata>> {
        self.executor_view
            .get_module_state_value_metadata(state_key)
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<StateValueMetadata>> {
        self.executor_view
            .get_resource_state_value_metadata(state_key)
    }
}

// Allows to extract the view from `StorageAdapter`.
impl<'e, E: ExecutorView> AsExecutorView for StorageAdapter<'e, E> {
    fn as_executor_view(&self) -> &dyn ExecutorView {
        self.executor_view
    }
}

// Allows to extract the view from `StorageAdapter`.
impl<'e, E> AsResourceGroupView for StorageAdapter<'e, E> {
    fn as_resource_group_view(&self) -> &dyn ResourceGroupView {
        &self.resource_group_view
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use aptos_vm_types::resource_group_adapter::GroupSizeKind;

    // Expose a method to create a storage adapter with a provided group size kind.
    pub(crate) fn as_resolver_with_group_size_kind<S: StateView>(
        state_view: &S,
        group_size_kind: GroupSizeKind,
    ) -> StorageAdapter<S> {
        assert!(group_size_kind != GroupSizeKind::AsSum, "not yet supported");

        let (gas_feature_version, resource_group_charge_as_size_sum_enabled) = match group_size_kind
        {
            GroupSizeKind::AsSum => (12, true),
            GroupSizeKind::AsBlob => (10, false),
            GroupSizeKind::None => (1, false),
        };

        let group_adapter = ResourceGroupAdapter::new(
            // TODO[agg_v2](fix) add a converter for StateView for tests that implements ResourceGroupView
            None,
            state_view,
            gas_feature_version,
            resource_group_charge_as_size_sum_enabled,
        );
        StorageAdapter::new(state_view, 0, 0, group_adapter)
    }
}
