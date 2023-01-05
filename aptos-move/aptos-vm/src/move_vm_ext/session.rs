// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache, data_cache::MoveResolverWithVMMetadata,
    move_vm_ext::MoveResolverExt, transaction_metadata::TransactionMetadata,
};
use aptos_aggregator::{
    aggregator_extension::AggregatorID,
    delta_change_set::{serialize, DeltaChangeSet},
    transaction::ChangeSetExt,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_framework::natives::{
    aggregator_natives::{AggregatorChange, AggregatorChangeSet, NativeAggregatorContext},
    code::{NativeCodeContext, PublishRequest},
};
use aptos_gas::ChangeSetConfigs;
use aptos_types::{
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{ChangeSet, SignatureCheckedTransaction},
    write_set::{WriteOp, WriteSetMut},
};
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    effects::{
        AccountChangeSet, ChangeSet as MoveChangeSet, Event as MoveEvent, Op as MoveStorageOp,
    },
    language_storage::{ModuleId, StructTag, TypeTag},
    vm_status::{StatusCode, VMStatus},
};
use move_table_extension::{NativeTableContext, TableChange, TableChangeSet};
use move_vm_runtime::{move_vm::MoveVM, session::Session};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

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
    // For those runs that are not a transaction and the output of which won't be committed.
    Void,
}

impl SessionId {
    pub fn txn(txn: &SignatureCheckedTransaction) -> Self {
        Self::txn_meta(&TransactionMetadata::new(&txn.clone().into_inner()))
    }

    pub fn txn_meta(txn_data: &TransactionMetadata) -> Self {
        Self::Txn {
            sender: txn_data.sender,
            sequence_number: txn_data.sequence_number,
            script_hash: txn_data.script_hash.clone(),
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

    pub fn void() -> Self {
        Self::Void
    }

    pub fn as_uuid(&self) -> HashValue {
        self.hash()
    }
}

pub struct SessionExt<'r, 'l, S> {
    inner: Session<'r, 'l, S>,
    remote: MoveResolverWithVMMetadata<'r, 'l, S>,
}

impl<'r, 'l, S> SessionExt<'r, 'l, S>
where
    S: MoveResolverExt,
{
    pub fn new(inner: Session<'r, 'l, S>, move_vm: &'l MoveVM, remote: &'r S) -> Self {
        Self {
            inner,
            remote: MoveResolverWithVMMetadata::new(remote, move_vm),
        }
    }

    pub fn finish(self) -> VMResult<SessionOutput> {
        let (change_set, events, mut extensions) = self.inner.finish_with_extensions()?;

        // The use of this implies that we could theoretically call unwrap with no consequences,
        // but using unwrap means the code panics if someone can come up with an attack.
        let common_error = PartialVMError::new(StatusCode::UNREACHABLE).finish(Location::Undefined);
        let mut change_set_grouped = MoveChangeSet::new();
        for (addr, account_changeset) in change_set.into_inner() {
            let mut resource_groups: BTreeMap<StructTag, AccountChangeSet> = BTreeMap::new();
            let mut account_changeset_grouped = AccountChangeSet::new();
            let (modules, resources) = account_changeset.into_inner();

            // * Separate the resource groups from the non-resource groups
            // * non-resource groups are kept as is
            // * resource groups are merged into the correct format as deltas to the source data
            //   * Remove resource group data from the deltas
            //   * Attempt to read the existing resource group data or create a new empty container
            //   * Apply the deltas to the resource group data
            // The process for translating Move deltas of resource groups to resources is
            // * Add -- insert element in container
            //   * If entry exists, Unreachable
            //   * If group exists, Modify
            //   * If group doesn't exist, Add
            // * Modify -- update element in container
            //   * If group or data doesn't exist, Unreachable
            //   * Otherwise modify
            // * Delete -- remove element from container
            //   * If group or data does't exist, Unreachable
            //   * If elements remain, Modify
            //   * Otherwise delete
            for (struct_tag, blob_op) in resources {
                let resource_group = self
                    .remote
                    .get_resource_group(&struct_tag)
                    .map_err(|_| common_error.clone())?;
                if let Some(resource_group) = resource_group {
                    resource_groups
                        .entry(resource_group)
                        .or_insert_with(AccountChangeSet::new)
                        .add_resource_op(struct_tag, blob_op)
                        .map_err(|_| common_error.clone())?;
                } else {
                    account_changeset_grouped
                        .add_resource_op(struct_tag, blob_op)
                        .map_err(|_| common_error.clone())?;
                }
            }

            for (resource_tag, resources) in resource_groups {
                let source_data = self
                    .remote
                    .get_resource_group_data(&addr, &resource_tag)
                    .map_err(|_| common_error.clone())?;
                let (mut source_data, create) = if let Some(source_data) = source_data {
                    let source_data =
                        bcs::from_bytes(&source_data).map_err(|_| common_error.clone())?;
                    (source_data, false)
                } else {
                    (BTreeMap::new(), true)
                };

                for (struct_tag, current_op) in resources.into_resources() {
                    match current_op {
                        MoveStorageOp::Delete => {
                            source_data
                                .remove(&struct_tag)
                                .ok_or_else(|| common_error.clone())?;
                        },
                        MoveStorageOp::Modify(new_data) => {
                            let data = source_data
                                .get_mut(&struct_tag)
                                .ok_or_else(|| common_error.clone())?;
                            *data = new_data;
                        },
                        MoveStorageOp::New(data) => {
                            let data = source_data.insert(struct_tag, data);
                            if data.is_some() {
                                return Err(common_error);
                            }
                        },
                    }
                }

                let op = if source_data.is_empty() {
                    MoveStorageOp::Delete
                } else if create {
                    MoveStorageOp::New(
                        bcs::to_bytes(&source_data).map_err(|_| common_error.clone())?,
                    )
                } else {
                    MoveStorageOp::Modify(
                        bcs::to_bytes(&source_data).map_err(|_| common_error.clone())?,
                    )
                };
                account_changeset_grouped
                    .add_resource_op(resource_tag, op)
                    .map_err(|_| common_error.clone())?;
            }

            for (name, blob_op) in modules {
                account_changeset_grouped
                    .add_module_op(name, blob_op)
                    .map_err(|_| common_error.clone())?;
            }

            change_set_grouped
                .add_account_changeset(addr, account_changeset_grouped)
                .map_err(|_| common_error.clone())?;
        }

        let table_context: NativeTableContext = extensions.remove();
        let table_change_set = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let aggregator_context: NativeAggregatorContext = extensions.remove();
        let aggregator_change_set = aggregator_context.into_change_set();

        Ok(SessionOutput {
            change_set: change_set_grouped,
            events,
            table_change_set,
            aggregator_change_set,
        })
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.get_native_extensions().get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
    }

    pub fn validate_resource_groups(&self, modules: &[CompiledModule]) -> VMResult<()> {
        for module in modules {
            let metadata = if let Some(metadata) = aptos_framework::get_module_metadata(module) {
                metadata
            } else {
                continue;
            };

            for attrs in metadata.struct_attributes.values() {
                let attr = if let Some(attr) = attrs.iter().find(|attr| attr.is_resource_group_member()) {
                    attr
                } else {
                    continue;
                };

                // This should be validated during loading of data.
                let group = attr.get_resource_group_member().ok_or_else(|| {
                    PartialVMError::new(StatusCode::UNREACHABLE).finish(Location::Undefined)
                })?;

                // Make sure the type is in the module metadata is cached.
                self.load_type(&TypeTag::Struct(Box::new(group.clone())))?;
                // This might not exist, in which case, it is a failure.
                let group_module_metadata = self
                    .remote
                    .get_module_metadata(group.module_id())
                    .ok_or_else(|| {
                        PartialVMError::new(StatusCode::LINKER_ERROR).finish(Location::Undefined)
                    })?;

                // This might not exist, in which case, it is a failure.
                let scope = group_module_metadata
                    .struct_attributes
                    .get(group.name.as_str())
                    .and_then(|container_metadata| {
                        container_metadata
                            .iter()
                            .find(|attr| attr.is_resource_group())
                    })
                    .and_then(|container| container.get_resource_group())
                    .ok_or_else(|| {
                        PartialVMError::new(StatusCode::LINKER_ERROR).finish(Location::Undefined)
                    })?;

                if !scope.are_equal_module_ids(&module.self_id(), &group.module_id()) {
                    return Err(
                        PartialVMError::new(StatusCode::LINKER_ERROR).finish(Location::Undefined)
                    );
                }
            }
        }
        Ok(())
    }
}

impl<'r, 'l, S> Deref for SessionExt<'r, 'l, S> {
    type Target = Session<'r, 'l, S>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'r, 'l, S> DerefMut for SessionExt<'r, 'l, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct SessionOutput {
    pub change_set: MoveChangeSet,
    pub events: Vec<MoveEvent>,
    pub table_change_set: TableChangeSet,
    pub aggregator_change_set: AggregatorChangeSet,
}

// TODO: Move this into the Move repo.
fn squash_table_change_sets(
    base: &mut TableChangeSet,
    other: TableChangeSet,
) -> Result<(), VMStatus> {
    base.new_tables.extend(other.new_tables);
    for removed_table in &base.removed_tables {
        base.new_tables.remove(removed_table);
    }
    // There's chance that a table is added in `self`, and an item is added to that table in
    // `self`, and later the item is deleted in `other`, netting to a NOOP for that item,
    // but this is an tricky edge case that we don't expect to happen too much, it doesn't hurt
    // too much to just keep the deletion. It's safe as long as we do it that way consistently.
    base.removed_tables.extend(other.removed_tables.into_iter());
    for (handle, changes) in other.changes.into_iter() {
        let my_changes = base.changes.entry(handle).or_insert(TableChange {
            entries: Default::default(),
        });
        my_changes.entries.extend(changes.entries.into_iter());
    }
    Ok(())
}

impl SessionOutput {
    pub fn into_change_set<C: AccessPathCache>(
        self,
        ap_cache: &mut C,
        configs: &ChangeSetConfigs,
    ) -> Result<ChangeSetExt, VMStatus> {
        use MoveStorageOp::*;
        let Self {
            change_set,
            events,
            table_change_set,
            aggregator_change_set,
        } = self;

        let mut write_set_mut = WriteSetMut::new(Vec::new());
        let mut delta_change_set = DeltaChangeSet::empty();

        for (addr, account_changeset) in change_set.into_inner() {
            let (modules, resources) = account_changeset.into_inner();
            for (struct_tag, blob_op) in resources {
                let ap = ap_cache.get_resource_path(addr, struct_tag);
                let op = match blob_op {
                    Delete => WriteOp::Deletion,
                    New(blob) => {
                        if configs.creation_as_modification() {
                            WriteOp::Modification(blob)
                        } else {
                            WriteOp::Creation(blob)
                        }
                    },
                    Modify(blob) => WriteOp::Modification(blob),
                };
                write_set_mut.insert((StateKey::AccessPath(ap), op))
            }

            for (name, blob_op) in modules {
                let ap = ap_cache.get_module_path(ModuleId::new(addr, name));
                let op = match blob_op {
                    Delete => WriteOp::Deletion,
                    New(blob) => WriteOp::Creation(blob),
                    Modify(blob) => WriteOp::Modification(blob),
                };

                write_set_mut.insert((StateKey::AccessPath(ap), op))
            }
        }

        for (handle, change) in table_change_set.changes {
            for (key, value_op) in change.entries {
                let state_key = StateKey::table_item(handle.into(), key);
                match value_op {
                    Delete => write_set_mut.insert((state_key, WriteOp::Deletion)),
                    New(bytes) => write_set_mut.insert((state_key, WriteOp::Creation(bytes))),
                    Modify(bytes) => {
                        write_set_mut.insert((state_key, WriteOp::Modification(bytes)))
                    },
                }
            }
        }

        for (id, change) in aggregator_change_set.changes {
            let AggregatorID { handle, key } = id;
            let key_bytes = key.0.to_vec();
            let state_key = StateKey::table_item(TableHandle::from(handle), key_bytes);

            match change {
                AggregatorChange::Write(value) => {
                    let write_op = WriteOp::Modification(serialize(&value));
                    write_set_mut.insert((state_key, write_op));
                },
                AggregatorChange::Merge(delta_op) => delta_change_set.insert((state_key, delta_op)),
                AggregatorChange::Delete => {
                    let write_op = WriteOp::Deletion;
                    write_set_mut.insert((state_key, write_op));
                },
            }
        }

        let write_set = write_set_mut
            .freeze()
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR))?;

        let events = events
            .into_iter()
            .map(|(guid, seq_num, ty_tag, blob)| {
                let key = bcs::from_bytes(guid.as_slice())
                    .map_err(|_| VMStatus::Error(StatusCode::EVENT_KEY_MISMATCH))?;
                Ok(ContractEvent::new(key, seq_num, ty_tag, blob))
            })
            .collect::<Result<Vec<_>, VMStatus>>()?;

        let change_set = ChangeSet::new(write_set, events, configs)?;
        Ok(ChangeSetExt::new(
            delta_change_set,
            change_set,
            Arc::new(configs.clone()),
        ))
    }

    pub fn squash(&mut self, other: Self) -> Result<(), VMStatus> {
        self.change_set
            .squash(other.change_set)
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR))?;
        self.events.extend(other.events.into_iter());

        // Squash the table changes.
        squash_table_change_sets(&mut self.table_change_set, other.table_change_set)?;

        // Squash aggregator changes.
        self.aggregator_change_set
            .squash(other.aggregator_change_set)?;

        Ok(())
    }
}
