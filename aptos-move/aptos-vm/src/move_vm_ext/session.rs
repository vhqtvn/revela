// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::get_resource_group_from_metadata,
    move_vm_ext::{write_op_converter::WriteOpConverter, AptosMoveResolver},
    transaction_metadata::TransactionMetadata,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_framework::natives::{
    aggregator_natives::{AggregatorChangeSet, AggregatorChangeV1, NativeAggregatorContext},
    code::{NativeCodeContext, PublishRequest},
    event::NativeEventContext,
};
use aptos_table_natives::{NativeTableContext, TableChangeSet};
use aptos_types::{
    access_path::AccessPath, block_metadata::BlockMetadata, contract_event::ContractEvent,
    on_chain_config::Features, state_store::state_key::StateKey,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, Changes, Op as MoveStorageOp},
    language_storage::{ModuleId, StructTag},
    value::MoveTypeLayout,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{move_vm::MoveVM, session::Session};
use move_vm_types::values::Value;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub(crate) enum ResourceGroupChangeSet {
    // Merged resource groups op.
    V0(BTreeMap<StateKey, MoveStorageOp<BytesWithResourceLayout>>),
    // Granular ops to individual resources within a group.
    V1(BTreeMap<StateKey, BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>>),
}
type AccountChangeSet = AccountChanges<Bytes, BytesWithResourceLayout>;
type ChangeSet = Changes<Bytes, BytesWithResourceLayout>;
pub type BytesWithResourceLayout = (Bytes, Option<Arc<MoveTypeLayout>>);

#[derive(BCSCryptoHash, CryptoHasher, Deserialize, Serialize)]
pub enum SessionId {
    Txn {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    BlockMeta {
        // block id
        id: HashValue,
    },
    Genesis {
        // id to identify this specific genesis build
        id: HashValue,
    },
    Prologue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    Epilogue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    // For those runs that are not a transaction and the output of which won't be committed.
    Void,
    RunOnAbort {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
}

impl SessionId {
    pub fn txn_meta(txn_metadata: &TransactionMetadata) -> Self {
        Self::Txn {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn genesis(id: HashValue) -> Self {
        Self::Genesis { id }
    }

    pub fn block_meta(block_meta: &BlockMetadata) -> Self {
        Self::BlockMeta {
            id: block_meta.id(),
        }
    }

    pub fn prologue_meta(txn_metadata: &TransactionMetadata) -> Self {
        Self::Prologue {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn run_on_abort(txn_metadata: &TransactionMetadata) -> Self {
        Self::RunOnAbort {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn epilogue_meta(txn_metadata: &TransactionMetadata) -> Self {
        Self::Epilogue {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn void() -> Self {
        Self::Void
    }

    pub fn as_uuid(&self) -> HashValue {
        self.hash()
    }
}

pub struct SessionExt<'r, 'l> {
    inner: Session<'r, 'l>,
    remote: &'r dyn AptosMoveResolver,
    features: Arc<Features>,
}

impl<'r, 'l> SessionExt<'r, 'l> {
    pub fn new(
        inner: Session<'r, 'l>,
        remote: &'r dyn AptosMoveResolver,
        features: Arc<Features>,
    ) -> Self {
        Self {
            inner,
            remote,
            features,
        }
    }

    pub fn finish(self, configs: &ChangeSetConfigs) -> VMResult<VMChangeSet> {
        let move_vm = self.inner.get_move_vm();

        let resource_converter = |value: Value,
                                  layout: MoveTypeLayout,
                                  has_aggregator_lifting: bool|
         -> PartialVMResult<BytesWithResourceLayout> {
            value
                .simple_serialize(&layout)
                .map(Into::into)
                .map(|bytes| (bytes, has_aggregator_lifting.then_some(Arc::new(layout))))
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                        .with_message(format!("Error when serializing resource {}.", value))
                })
        };
        let (change_set, mut extensions) = self
            .inner
            .finish_with_extensions_with_custom_effects(&resource_converter)?;

        let (change_set, resource_group_change_set) =
            Self::split_and_merge_resource_groups(move_vm, self.remote, change_set)?;

        let table_context: NativeTableContext = extensions.remove();
        let table_change_set = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let aggregator_context: NativeAggregatorContext = extensions.remove();
        let aggregator_change_set = aggregator_context
            .into_change_set()
            .map_err(|e| PartialVMError::from(e).finish(Location::Undefined))?;

        let event_context: NativeEventContext = extensions.remove();
        let events = event_context.into_events();

        let woc = WriteOpConverter::new(
            self.remote,
            self.features.is_storage_slot_metadata_enabled(),
        );

        let change_set = Self::convert_change_set(
            &woc,
            change_set,
            resource_group_change_set,
            events,
            table_change_set,
            aggregator_change_set,
            configs,
        )
        .map_err(|status| PartialVMError::new(status.status_code()).finish(Location::Undefined))?;

        Ok(change_set)
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.get_native_extensions().get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
    }

    fn populate_v0_resource_group_change_set(
        change_set: &mut BTreeMap<StateKey, MoveStorageOp<BytesWithResourceLayout>>,
        state_key: StateKey,
        mut source_data: BTreeMap<StructTag, Bytes>,
        resources: BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
    ) -> VMResult<()> {
        let common_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("populate v0 resource group change set error".to_string())
                .finish(Location::Undefined)
        };

        let create = source_data.is_empty();

        for (struct_tag, current_op) in resources {
            match current_op {
                MoveStorageOp::Delete => {
                    source_data.remove(&struct_tag).ok_or_else(common_error)?;
                },
                MoveStorageOp::Modify((new_data, _)) => {
                    let data = source_data.get_mut(&struct_tag).ok_or_else(common_error)?;
                    *data = new_data;
                },
                MoveStorageOp::New((data, _)) => {
                    let data = source_data.insert(struct_tag, data);
                    if data.is_some() {
                        return Err(common_error());
                    }
                },
            }
        }

        let op = if source_data.is_empty() {
            MoveStorageOp::Delete
        } else if create {
            MoveStorageOp::New((
                bcs::to_bytes(&source_data)
                    .map_err(|_| common_error())?
                    .into(),
                None,
            ))
        } else {
            MoveStorageOp::Modify((
                bcs::to_bytes(&source_data)
                    .map_err(|_| common_error())?
                    .into(),
                None,
            ))
        };
        change_set.insert(state_key, op);
        Ok(())
    }

    /// * Separate the resource groups from the non-resource.
    /// * non-resource groups are kept as is
    /// * resource groups are merged into the correct format as deltas to the source data
    ///   * Remove resource group data from the deltas
    ///   * Attempt to read the existing resource group data or create a new empty container
    ///   * Apply the deltas to the resource group data
    /// The process for translating Move deltas of resource groups to resources is
    /// * Add -- insert element in container
    ///   * If entry exists, Unreachable
    ///   * If group exists, Modify
    ///   * If group doesn't exist, Add
    /// * Modify -- update element in container
    ///   * If group or data doesn't exist, Unreachable
    ///   * Otherwise modify
    /// * Delete -- remove element from container
    ///   * If group or data does't exist, Unreachable
    ///   * If elements remain, Modify
    ///   * Otherwise delete
    ///
    /// V1 Resource group change set behavior keeps ops for individual resources separate, not
    /// merging them into the a single op corresponding to the whole resource group (V0).
    /// TODO[agg_v2](fix) Resource groups are currently not handled correctly in terms of propagating MoveTypeLayout
    fn split_and_merge_resource_groups(
        runtime: &MoveVM,
        remote: &dyn AptosMoveResolver,
        change_set: ChangeSet,
    ) -> VMResult<(ChangeSet, ResourceGroupChangeSet)> {
        // The use of this implies that we could theoretically call unwrap with no consequences,
        // but using unwrap means the code panics if someone can come up with an attack.
        let common_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("split_and_merge_resource_groups error".to_string())
                .finish(Location::Undefined)
        };
        let mut change_set_filtered = ChangeSet::new();

        let mut maybe_resource_group_cache = remote.release_resource_group_cache().map(|v| {
            v.into_iter()
                .map(|(k, v)| (k, v.into_iter().collect::<BTreeMap<_, _>>()))
                .collect::<BTreeMap<_, _>>()
        });
        let mut resource_group_change_set = if maybe_resource_group_cache.is_some() {
            ResourceGroupChangeSet::V0(BTreeMap::new())
        } else {
            ResourceGroupChangeSet::V1(BTreeMap::new())
        };
        for (addr, account_changeset) in change_set.into_inner() {
            let mut resource_groups: BTreeMap<
                StructTag,
                BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
            > = BTreeMap::new();
            let mut resources_filtered = BTreeMap::new();
            let (modules, resources) = account_changeset.into_inner();

            for (struct_tag, blob_op) in resources {
                let resource_group_tag = runtime
                    .with_module_metadata(&struct_tag.module_id(), |md| {
                        get_resource_group_from_metadata(&struct_tag, md)
                    });

                if let Some(resource_group_tag) = resource_group_tag {
                    if resource_groups
                        .entry(resource_group_tag)
                        .or_insert_with(BTreeMap::new)
                        .insert(struct_tag, blob_op)
                        .is_some()
                    {
                        return Err(common_error());
                    }
                } else {
                    resources_filtered.insert(struct_tag, blob_op);
                }
            }

            change_set_filtered
                .add_account_changeset(
                    addr,
                    AccountChangeSet::from_modules_resources(modules, resources_filtered),
                )
                .map_err(|_| common_error())?;

            for (resource_group_tag, resources) in resource_groups {
                let state_key = StateKey::access_path(AccessPath::resource_group_access_path(
                    addr,
                    resource_group_tag,
                ));
                match &mut resource_group_change_set {
                    ResourceGroupChangeSet::V0(v0_changes) => {
                        let source_data = maybe_resource_group_cache
                            .as_mut()
                            .expect("V0 cache must be set")
                            .remove(&state_key)
                            .unwrap_or_default();
                        Self::populate_v0_resource_group_change_set(
                            v0_changes,
                            state_key,
                            source_data,
                            resources,
                        )?;
                    },
                    ResourceGroupChangeSet::V1(v1_changes) => {
                        // Maintain the behavior of failing the transaction on resource
                        // group member existence invariants.
                        for (struct_tag, current_op) in resources.iter() {
                            let exists = remote
                                .resource_exists_in_group(&state_key, struct_tag)
                                .map_err(|_| common_error())?;
                            if matches!(current_op, MoveStorageOp::New(_)) == exists {
                                // Deletion and Modification require resource to exist,
                                // while creation requires the resource to not exist.
                                return Err(common_error());
                            }
                        }
                        v1_changes.insert(state_key, resources);
                    },
                }
            }
        }

        Ok((change_set_filtered, resource_group_change_set))
    }

    pub(crate) fn convert_change_set(
        woc: &WriteOpConverter,
        change_set: ChangeSet,
        resource_group_change_set: ResourceGroupChangeSet,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        table_change_set: TableChangeSet,
        aggregator_change_set: AggregatorChangeSet,
        configs: &ChangeSetConfigs,
    ) -> Result<VMChangeSet, VMStatus> {
        let mut resource_write_set = BTreeMap::new();
        let mut resource_group_write_set = BTreeMap::new();
        let mut module_write_set = BTreeMap::new();
        let mut aggregator_v1_write_set = BTreeMap::new();
        let mut aggregator_v1_delta_set = BTreeMap::new();

        for (addr, account_changeset) in change_set.into_inner() {
            let (modules, resources) = account_changeset.into_inner();
            for (struct_tag, blob_and_layout_op) in resources {
                let state_key = StateKey::access_path(
                    AccessPath::resource_access_path(addr, struct_tag)
                        .unwrap_or_else(|_| AccessPath::undefined()),
                );
                let op = woc.convert_resource(
                    &state_key,
                    blob_and_layout_op,
                    configs.legacy_resource_creation_as_modification(),
                )?;

                resource_write_set.insert(state_key, op);
            }

            for (name, blob_op) in modules {
                let state_key = StateKey::access_path(AccessPath::from(&ModuleId::new(addr, name)));
                let op = woc.convert_module(&state_key, blob_op, false)?;
                module_write_set.insert(state_key, op);
            }
        }

        match resource_group_change_set {
            ResourceGroupChangeSet::V0(v0_changes) => {
                for (state_key, blob_op) in v0_changes {
                    let op = woc.convert_resource(&state_key, blob_op, false)?;
                    resource_write_set.insert(state_key, op);
                }
            },
            ResourceGroupChangeSet::V1(v1_changes) => {
                for (state_key, resources) in v1_changes {
                    let group_write = woc.convert_resource_group_v1(&state_key, resources)?;
                    resource_group_write_set.insert(state_key, group_write);
                }
            },
        }

        for (handle, change) in table_change_set.changes {
            for (key, value_op) in change.entries {
                let state_key = StateKey::table_item(handle.into(), key);
                let op = woc.convert_resource(&state_key, value_op, false)?;
                resource_write_set.insert(state_key, op);
            }
        }

        for (state_key, change) in aggregator_change_set.aggregator_v1_changes {
            match change {
                AggregatorChangeV1::Write(value) => {
                    let write_op = woc.convert_aggregator_modification(&state_key, value)?;
                    aggregator_v1_write_set.insert(state_key, write_op);
                },
                AggregatorChangeV1::Merge(delta_op) => {
                    aggregator_v1_delta_set.insert(state_key, delta_op);
                },
                AggregatorChangeV1::Delete => {
                    let write_op =
                        woc.convert_aggregator(&state_key, MoveStorageOp::Delete, false)?;
                    aggregator_v1_write_set.insert(state_key, write_op);
                },
            }
        }

        // We need to remove values that are already in the writes.
        let reads_needing_exchange = aggregator_change_set
            .reads_needing_exchange
            .into_iter()
            .filter(|(state_key, _)| !resource_write_set.contains_key(state_key))
            .collect();

        let group_reads_needing_change = aggregator_change_set
            .group_reads_needing_exchange
            .into_iter()
            .filter(|(state_key, _)| !resource_group_write_set.contains_key(state_key))
            .collect();

        VMChangeSet::new_expanded(
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            aggregator_change_set.delayed_field_changes,
            reads_needing_exchange,
            group_reads_needing_change,
            events,
            configs,
        )
    }
}

impl<'r, 'l> Deref for SessionExt<'r, 'l> {
    type Target = Session<'r, 'l>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'r, 'l> DerefMut for SessionExt<'r, 'l> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
