// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::resolver::{ResourceGroupView, TResourceGroupView, TResourceView};
use anyhow::Error;
use aptos_types::state_store::state_key::StateKey;
use bytes::Bytes;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
};

/// Corresponding to different gas features, methods for counting the 'size' of a
/// resource group. None leads to 0, while AsBlob provides the group size as the
/// size of the serialized blob of the BTreeMap corresponding to the group.
/// For AsSum, the size is summed for each resource contained in the group (of
/// the resource blob, and its corresponding tag, when serialized)
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum GroupSizeKind {
    None,
    AsBlob,
    AsSum,
}

impl GroupSizeKind {
    pub fn from_gas_feature_version(gas_feature_version: u64) -> Self {
        if gas_feature_version >= 9 {
            if gas_feature_version >= 12 {
                GroupSizeKind::AsSum
            } else {
                // Keep old caching behavior for replay.
                GroupSizeKind::AsBlob
            }
        } else {
            GroupSizeKind::None
        }
    }
}

/// Handles the resolution of ResourceGroupView interfaces. If the gas feature version is
/// sufficiently new (corresponding to GroupSizeKind::AsSum), maybe_resource_group_view will
/// be used first, if set (this way, block executor provides the new resolution behavior).
///
/// If gas feature corresponding to AsSum is not enabled, maybe_resource_group_view is set
/// to None in any case, as block executor does not support older gas charging behavior.
/// When maybe_resource_group_view is None, group view resolution happens based on the
/// resource view interfaces, with an underlying cache. The cache is for efficiency, but
/// also released to the session for older feature versions (needed to prepare VM output).
///
/// Currently, maybe_resource_group_view is always set to None, and cache always released.
/// There are TODOs in the code for enabling the new behavior.
pub struct ResourceGroupAdapter<'r> {
    maybe_resource_group_view: Option<&'r dyn ResourceGroupView>,
    resource_view: &'r dyn TResourceView<Key = StateKey, Layout = MoveTypeLayout>,
    group_size_kind: GroupSizeKind,
    group_cache: RefCell<HashMap<StateKey, (BTreeMap<StructTag, Bytes>, u64)>>,
}

impl<'r> ResourceGroupAdapter<'r> {
    pub fn new(
        _maybe_resource_group_view: Option<&'r dyn ResourceGroupView>,
        resource_view: &'r dyn TResourceView<Key = StateKey, Layout = MoveTypeLayout>,
        gas_feature_version: u64,
    ) -> Self {
        let group_size_kind = GroupSizeKind::from_gas_feature_version(gas_feature_version);

        Self {
            // TODO: when ResourceGroupView is implemented by block executor, provide
            // (group_size_kind == GroupSizeKind::AsSum).then(|| maybe_resource_group_view)
            maybe_resource_group_view: None,
            resource_view,
            group_size_kind,
            group_cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn group_size_kind(&self) -> GroupSizeKind {
        self.group_size_kind.clone()
    }

    // Ensures that the resource group at state_key is cached in self.group_cache. Ok(true)
    // means the resource was already cached, while Ok(false) means it just got cached.
    fn load_to_cache(&self, group_key: &StateKey) -> anyhow::Result<bool> {
        let already_cached = self.group_cache.borrow().contains_key(group_key);
        if already_cached {
            return Ok(true);
        }

        let group_data = self.resource_view.get_resource_bytes(group_key, None)?;
        let (group_data, blob_len): (BTreeMap<StructTag, Bytes>, u64) = group_data.map_or_else(
            || Ok::<_, Error>((BTreeMap::new(), 0)),
            |group_data_blob| {
                let group_data = bcs::from_bytes(&group_data_blob)
                    .map_err(|_| anyhow::Error::msg("Resource group deserialization error"))?;
                Ok((group_data, group_data_blob.len() as u64))
            },
        )?;

        let group_size = match self.group_size_kind {
            GroupSizeKind::None => 0,
            GroupSizeKind::AsBlob => blob_len,
            GroupSizeKind::AsSum => group_data
                .iter()
                .try_fold(0, |len, (tag, res)| {
                    let delta = bcs::serialized_size(tag)? + res.len();
                    Ok(len + delta as u64)
                })
                .map_err(|_: Error| {
                    anyhow::Error::msg("Resource group member tag serialization error")
                })?,
        };
        self.group_cache
            .borrow_mut()
            .insert(group_key.clone(), (group_data, group_size));
        Ok(false)
    }
}

impl TResourceGroupView for ResourceGroupAdapter<'_> {
    type GroupKey = StateKey;
    type Layout = MoveTypeLayout;
    type ResourceTag = StructTag;

    fn resource_group_size(&self, group_key: &Self::GroupKey) -> anyhow::Result<u64> {
        if self.group_size_kind == GroupSizeKind::None {
            return Ok(0);
        }

        if let Some(group_view) = self.maybe_resource_group_view {
            return group_view.resource_group_size(group_key);
        }

        self.load_to_cache(group_key)?;
        Ok(self
            .group_cache
            .borrow()
            .get(group_key)
            .expect("Must be cached")
            .1)
    }

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> anyhow::Result<Option<Bytes>> {
        if let Some(group_view) = self.maybe_resource_group_view {
            return group_view.get_resource_from_group(group_key, resource_tag, maybe_layout);
        }

        self.load_to_cache(group_key)?;
        Ok(self
            .group_cache
            .borrow()
            .get(group_key)
            .expect("Must be cached")
            .0 // btreemap
            .get(resource_tag)
            .cloned())
    }

    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>> {
        // TODO: once block executor can handle the new VMChangeSet, return None when
        // group_size_kind is GroupSizeKind::AsSum, which will also switch output of session
        // to a newer format and correspondingly, to the new (AsSum) gas charging for it.
        // Note: should still flush the cache, even if returning None.
        Some(
            self.group_cache
                .borrow_mut()
                .drain()
                .map(|(k, v)| (k, v.0))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{mock_tag_0, mock_tag_1, mock_tag_2};
    use aptos_state_view::TStateView;
    use aptos_types::state_store::{
        state_storage_usage::StateStorageUsage, state_value::StateValue,
    };
    use claims::{assert_gt, assert_lt, assert_none, assert_ok_eq, assert_some, assert_some_eq};
    use std::cmp::max;

    struct MockGroup {
        blob: Vec<u8>,
        size_as_sum: usize,
    }

    impl MockGroup {
        fn new(contents: BTreeMap<StructTag, Vec<u8>>) -> Self {
            let mut size_as_sum = 0;
            for (tag, v) in &contents {
                // Compute size indirectly, by first serializing.
                let serialized_tag = bcs::to_bytes(&tag).unwrap();
                size_as_sum += v.len() + serialized_tag.len();
            }
            let blob = bcs::to_bytes(&contents).unwrap();

            Self { blob, size_as_sum }
        }
    }

    struct MockStateView {
        group: HashMap<StateKey, MockGroup>,
    }

    impl MockStateView {
        fn new() -> Self {
            let mut group = HashMap::new();

            let key_0 = StateKey::raw(vec![0]);
            let key_1 = StateKey::raw(vec![1]);

            // for testing purposes, o.w. state view should never contain an empty map.
            group.insert(key_0, MockGroup::new(BTreeMap::new()));
            group.insert(
                key_1,
                MockGroup::new(BTreeMap::from([
                    (mock_tag_0(), vec![0; 1000]),
                    (mock_tag_1(), vec![1; 500]),
                ])),
            );

            Self { group }
        }
    }

    impl TStateView for MockStateView {
        type Key = StateKey;

        fn get_state_value(&self, state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
            Ok(self
                .group
                .get(state_key)
                .map(|entry| StateValue::new_legacy(entry.blob.clone().into())))
        }

        fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
            unimplemented!();
        }
    }

    #[test]
    fn group_size_kind_from_gas_version() {
        for i in 0..9 {
            assert_eq!(
                GroupSizeKind::from_gas_feature_version(i),
                GroupSizeKind::None
            );
        }

        for i in 9..12 {
            assert_eq!(
                GroupSizeKind::from_gas_feature_version(i),
                GroupSizeKind::AsBlob
            );
        }

        for i in 12..20 {
            assert_eq!(
                GroupSizeKind::from_gas_feature_version(i),
                GroupSizeKind::AsSum
            );
        }
    }

    #[test]
    fn load_to_cache() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 3);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::None);

        let key_1 = StateKey::raw(vec![1]);
        let tag_0 = mock_tag_0();

        assert_ok_eq!(adapter.load_to_cache(&key_1), false);
        let _ = adapter.get_resource_from_group(&key_1, &tag_0, None);
        assert_ok_eq!(adapter.load_to_cache(&key_1), true);
    }

    #[test]
    fn test_get_resource_by_tag() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 5);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = mock_tag_0();
        let tag_1 = mock_tag_1();
        let tag_2 = mock_tag_2();

        // key_0 / tag_0 does not exist.
        assert_none!(adapter
            .get_resource_from_group(&key_0, &tag_0, None)
            .unwrap());

        assert_some_eq!(
            adapter
                .get_resource_from_group(&key_1, &tag_0, None)
                .unwrap(),
            vec![0; 1000]
        );

        // key_2 / tag_1 does not exist.
        assert_none!(adapter
            .get_resource_from_group(&key_2, &tag_1, None)
            .unwrap());

        let key_1_blob = &state_view.group.get(&key_1).unwrap().blob;

        // Release the cache to test contents.
        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 3);
        assert!(cache.get(&key_0).expect("Must be Some(..)").is_empty());
        assert!(cache.get(&key_2).expect("Must be Some(..)").is_empty());
        let cache_key_1_contents = cache.get(&key_1).unwrap();
        assert_eq!(bcs::to_bytes(&cache_key_1_contents).unwrap(), *key_1_blob);

        assert_some_eq!(
            adapter
                .get_resource_from_group(&key_1, &tag_1, None)
                .unwrap(),
            vec![1; 500]
        );

        assert_none!(adapter
            .get_resource_from_group(&key_1, &tag_2, None)
            .unwrap());

        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 1);
        let cache_key_1_contents = cache.get(&key_1).unwrap();
        assert_eq!(bcs::to_bytes(&cache_key_1_contents).unwrap(), *key_1_blob);
    }

    #[test]
    fn size_as_blob_len() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 9);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::AsBlob);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let key_0_blob_len = state_view.group.get(&key_0).unwrap().blob.len() as u64;
        let key_1_blob_len = state_view.group.get(&key_1).unwrap().blob.len() as u64;

        assert_ok_eq!(adapter.resource_group_size(&key_1), key_1_blob_len);
        // Release the cache to test contents.
        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        assert_ok_eq!(adapter.resource_group_size(&key_0), key_0_blob_len);
        assert_ok_eq!(adapter.resource_group_size(&key_1), key_1_blob_len);
        assert_ok_eq!(adapter.resource_group_size(&key_2), 0);

        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 3);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));
    }

    #[test]
    fn size_as_sum() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 12);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::AsSum);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let key_0_size_as_sum = state_view.group.get(&key_0).unwrap().size_as_sum as u64;
        let key_1_size_as_sum = state_view.group.get(&key_1).unwrap().size_as_sum as u64;

        assert_ok_eq!(adapter.resource_group_size(&key_1), key_1_size_as_sum);
        // Release the cache via trait method and test contents.
        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 1);
        assert_some!(cache.get(&key_1));

        assert_ok_eq!(adapter.resource_group_size(&key_0), key_0_size_as_sum);
        assert_ok_eq!(adapter.resource_group_size(&key_1), key_1_size_as_sum);
        assert_ok_eq!(adapter.resource_group_size(&key_2), 0);

        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 3);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));

        // Sanity check the size numbers, at the time of writing the test 1582 and 1587.
        let key_1_blob_size = state_view.group.get(&key_1).unwrap().blob.len() as u64;
        assert_lt!(
            key_1_size_as_sum,
            key_1_blob_size,
            "size as sum must be less than BTreeMap blob size",
        );
        assert_gt!(
            key_1_size_as_sum,
            max(1000, key_1_blob_size - 100),
            "size as sum may not be too small"
        );
    }

    #[test]
    fn size_as_none() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 8);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        assert_ok_eq!(adapter.resource_group_size(&key_1), 0);
        // Test releasing the cache via trait method.
        let cache = adapter.release_group_cache().unwrap();
        // GroupSizeKind::None does not cache on size queries.
        assert_eq!(cache.len(), 0);

        assert_ok_eq!(adapter.resource_group_size(&key_0), 0);
        assert_ok_eq!(adapter.resource_group_size(&key_1), 0);
        assert_ok_eq!(adapter.resource_group_size(&key_2), 0);

        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn exists_resource_in_group() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 0);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = mock_tag_0();
        let tag_1 = mock_tag_1();
        let tag_2 = mock_tag_2();

        assert_ok_eq!(adapter.resource_exists_in_group(&key_1, &tag_0), true);
        assert_ok_eq!(adapter.resource_exists_in_group(&key_1, &tag_1), true);
        assert_ok_eq!(adapter.resource_exists_in_group(&key_2, &tag_2), false);
        // Release the cache to test contents.
        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));

        assert_ok_eq!(adapter.resource_exists_in_group(&key_0, &tag_1), false);
        assert_ok_eq!(adapter.resource_exists_in_group(&key_1, &tag_2), false);

        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
    }

    #[test]
    fn resource_size_in_group() {
        let state_view = MockStateView::new();
        let adapter = ResourceGroupAdapter::new(None, &state_view, 3);
        assert_eq!(adapter.group_size_kind, GroupSizeKind::None);

        let key_0 = StateKey::raw(vec![0]);
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);
        let tag_0 = mock_tag_0();
        let tag_1 = mock_tag_1();
        let tag_2 = mock_tag_2();

        let key_1_tag_0_len = adapter
            .get_resource_from_group(&key_1, &tag_0, None)
            .unwrap()
            .unwrap()
            .len() as u64;
        let key_1_tag_1_len = adapter
            .get_resource_from_group(&key_1, &tag_1, None)
            .unwrap()
            .unwrap()
            .len() as u64;

        assert_ok_eq!(
            adapter.resource_size_in_group(&key_1, &tag_0),
            key_1_tag_0_len
        );
        assert_ok_eq!(
            adapter.resource_size_in_group(&key_1, &tag_1),
            key_1_tag_1_len
        );
        assert_ok_eq!(adapter.resource_size_in_group(&key_2, &tag_2), 0);
        // Release the cache to test contents.
        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_1));
        assert_some!(cache.get(&key_2));

        assert_ok_eq!(adapter.resource_size_in_group(&key_0, &tag_1), 0);
        assert_ok_eq!(adapter.resource_size_in_group(&key_1, &tag_2), 0);

        let cache = adapter.release_group_cache().unwrap();
        assert_eq!(cache.len(), 2);
        assert_some!(cache.get(&key_0));
        assert_some!(cache.get(&key_1));
    }
}
