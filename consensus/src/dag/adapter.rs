// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensusdb::{
        CertifiedNodeSchema, ConsensusDB, DagVoteSchema, NodeSchema, OrderedAnchorIdSchema,
    },
    dag::{
        storage::{CommitEvent, DAGStorage},
        CertifiedNode, Node, NodeId, Vote,
    },
    experimental::buffer_manager::OrderedBlocks,
};
use anyhow::{anyhow, bail};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload, Round},
    executed_block::ExecutedBlock,
};
use aptos_crypto::HashValue;
use aptos_executor_types::StateComputeResult;
use aptos_logger::error;
use aptos_storage_interface::{DbReader, Order};
use aptos_types::{
    account_config::{new_block_event_key, NewBlockEvent},
    aggregate_signature::AggregateSignature,
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use async_trait::async_trait;
use futures_channel::mpsc::UnboundedSender;
use std::{collections::HashMap, sync::Arc};

#[async_trait]
pub trait Notifier: Send {
    fn send_ordered_nodes(
        &mut self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()>;

    async fn send_epoch_change(&self, proof: EpochChangeProof);

    async fn send_commit_proof(&self, ledger_info: LedgerInfoWithSignatures);
}
pub struct NotifierAdapter {
    executor_channel: UnboundedSender<OrderedBlocks>,
    storage: Arc<dyn DAGStorage>,
}

impl NotifierAdapter {
    pub fn new(
        executor_channel: UnboundedSender<OrderedBlocks>,
        storage: Arc<dyn DAGStorage>,
    ) -> Self {
        Self {
            executor_channel,
            storage,
        }
    }
}

#[async_trait]
impl Notifier for NotifierAdapter {
    fn send_ordered_nodes(
        &mut self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()> {
        let anchor = ordered_nodes.last().unwrap();
        let anchor_id = anchor.id();
        let epoch = anchor.epoch();
        let round = anchor.round();
        let timestamp = anchor.metadata().timestamp();
        let author = *anchor.author();
        let mut payload = Payload::empty(!anchor.payload().is_direct());
        let mut node_digests = vec![];
        for node in &ordered_nodes {
            payload.extend(node.payload().clone());
            node_digests.push(node.digest());
        }
        // TODO: we may want to split payload into multiple blocks
        let block = ExecutedBlock::new(
            Block::new_for_dag(epoch, round, timestamp, payload, author, failed_author)?,
            StateComputeResult::new_dummy(),
        );
        let block_info = block.block_info();
        let storage = self.storage.clone();
        Ok(self.executor_channel.unbounded_send(OrderedBlocks {
            ordered_blocks: vec![block],
            ordered_proof: LedgerInfoWithSignatures::new(
                LedgerInfo::new(block_info, anchor.digest()),
                AggregateSignature::empty(),
            ),
            callback: Box::new(
                move |_committed_blocks: &[Arc<ExecutedBlock>],
                      _commit_decision: LedgerInfoWithSignatures| {
                    // TODO: this doesn't really work since not every block will trigger a callback,
                    // we need to update the buffer manager to invoke all callbacks instead of only last one
                    if let Err(e) = storage
                        .delete_certified_nodes(node_digests)
                        .and_then(|_| storage.delete_ordered_anchor_ids(vec![anchor_id]))
                    {
                        error!(
                            "Failed to garbage collect committed nodes and anchor: {:?}",
                            e
                        );
                    }
                },
            ),
        })?)
    }

    async fn send_epoch_change(&self, _proof: EpochChangeProof) {
        todo!()
    }

    async fn send_commit_proof(&self, _ledger_info: LedgerInfoWithSignatures) {
        todo!()
    }
}

pub struct StorageAdapter {
    epoch: u64,
    epoch_to_validators: HashMap<u64, Vec<Author>>,
    consensus_db: Arc<ConsensusDB>,
    aptos_db: Arc<dyn DbReader>,
}

impl StorageAdapter {
    pub fn new(
        epoch: u64,
        epoch_to_validators: HashMap<u64, Vec<Author>>,
        consensus_db: Arc<ConsensusDB>,
        aptos_db: Arc<dyn DbReader>,
    ) -> Self {
        Self {
            epoch,
            epoch_to_validators,
            consensus_db,
            aptos_db,
        }
    }

    pub fn bitvec_to_validators(
        validators: &[Author],
        bitvec: &BitVec,
    ) -> anyhow::Result<Vec<Author>> {
        if BitVec::required_buckets(validators.len() as u16) != bitvec.num_buckets() {
            bail!(
                "bitvec bucket {} does not match validators len {}",
                bitvec.num_buckets(),
                validators.len()
            );
        }

        Ok(validators
            .iter()
            .enumerate()
            .filter_map(|(index, validator)| {
                if bitvec.is_set(index as u16) {
                    Some(*validator)
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn indices_to_validators(
        validators: &[Author],
        indices: &[u64],
    ) -> anyhow::Result<Vec<Author>> {
        indices
            .iter()
            .map(|index| {
                usize::try_from(*index)
                    .map_err(|_err| anyhow!("index {} out of bounds", index))
                    .and_then(|index| {
                        validators.get(index).cloned().ok_or(anyhow!(
                            "index {} is larger than number of validators {}",
                            index,
                            validators.len()
                        ))
                    })
            })
            .collect()
    }

    fn convert(&self, new_block_event: NewBlockEvent) -> anyhow::Result<CommitEvent> {
        let validators = &self.epoch_to_validators[&new_block_event.epoch()];
        Ok(CommitEvent::new(
            NodeId::new(
                new_block_event.epoch(),
                new_block_event.round(),
                new_block_event.proposer(),
            ),
            Self::bitvec_to_validators(
                validators,
                &new_block_event.previous_block_votes_bitvec().clone().into(),
            )?,
            Self::indices_to_validators(validators, new_block_event.failed_proposer_indices())?,
        ))
    }
}

impl DAGStorage for StorageAdapter {
    fn save_pending_node(&self, node: &Node) -> anyhow::Result<()> {
        Ok(self.consensus_db.put::<NodeSchema>(&(), node)?)
    }

    fn get_pending_node(&self) -> anyhow::Result<Option<Node>> {
        Ok(self.consensus_db.get::<NodeSchema>(&())?)
    }

    fn delete_pending_node(&self) -> anyhow::Result<()> {
        Ok(self.consensus_db.delete::<NodeSchema>(vec![()])?)
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()> {
        Ok(self.consensus_db.put::<DagVoteSchema>(node_id, vote)?)
    }

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>> {
        Ok(self.consensus_db.get_all::<DagVoteSchema>()?)
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.consensus_db.delete::<DagVoteSchema>(node_ids)?)
    }

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()> {
        Ok(self
            .consensus_db
            .put::<CertifiedNodeSchema>(&node.digest(), node)?)
    }

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>> {
        Ok(self.consensus_db.get_all::<CertifiedNodeSchema>()?)
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()> {
        Ok(self.consensus_db.delete::<CertifiedNodeSchema>(digests)?)
    }

    fn save_ordered_anchor_id(&self, node_id: &NodeId) -> anyhow::Result<()> {
        Ok(self
            .consensus_db
            .put::<OrderedAnchorIdSchema>(node_id, &())?)
    }

    fn get_ordered_anchor_ids(&self) -> anyhow::Result<Vec<(NodeId, ())>> {
        Ok(self.consensus_db.get_all::<OrderedAnchorIdSchema>()?)
    }

    fn delete_ordered_anchor_ids(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self
            .consensus_db
            .delete::<OrderedAnchorIdSchema>(node_ids)?)
    }

    fn get_latest_k_committed_events(&self, k: u64) -> anyhow::Result<Vec<CommitEvent>> {
        let latest_db_version = self.aptos_db.get_latest_version().unwrap_or(0);
        let mut commit_events = vec![];
        for event in self.aptos_db.get_events(
            &new_block_event_key(),
            u64::MAX,
            Order::Descending,
            k,
            latest_db_version,
        )? {
            let new_block_event = bcs::from_bytes::<NewBlockEvent>(event.event.event_data())?;
            if self
                .epoch_to_validators
                .contains_key(&new_block_event.epoch())
            {
                commit_events.push(self.convert(new_block_event)?);
            }
        }
        Ok(commit_events)
    }
}
