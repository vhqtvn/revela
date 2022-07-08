// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{consensusdb::ConsensusDB, epoch_manager::LivenessStorageData, error::DbError};
use anyhow::{format_err, Context, Result};
use aptos_config::config::NodeConfig;
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{
    epoch_change::EpochChangeProof, ledger_info::LedgerInfoWithSignatures, transaction::Version,
};
use consensus_types::{
    block::Block, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate, vote::Vote,
};
use std::{cmp::max, collections::HashSet, sync::Arc};
use storage_interface::DbReader;

/// PersistentLivenessStorage is essential for maintaining liveness when a node crashes.  Specifically,
/// upon a restart, a correct node will recover.  Even if all nodes crash, liveness is
/// guaranteed.
/// Blocks persisted are proposed but not yet committed.  The committed state is persisted
/// via StateComputer.
pub trait PersistentLivenessStorage: Send + Sync {
    /// Persist the blocks and quorum certs into storage atomically.
    fn save_tree(&self, blocks: Vec<Block>, quorum_certs: Vec<QuorumCert>) -> Result<()>;

    /// Delete the corresponding blocks and quorum certs atomically.
    fn prune_tree(&self, block_ids: Vec<HashValue>) -> Result<()>;

    /// Persist consensus' state
    fn save_vote(&self, vote: &Vote) -> Result<()>;

    /// Construct data that can be recovered from ledger
    fn recover_from_ledger(&self) -> LedgerRecoveryData;

    /// Construct necessary data to start consensus.
    fn start(&self) -> LivenessStorageData;

    /// Persist the highest 2chain timeout certificate for improved liveness - proof for other replicas
    /// to jump to this round
    fn save_highest_2chain_timeout_cert(
        &self,
        highest_timeout_cert: &TwoChainTimeoutCertificate,
    ) -> Result<()>;

    /// Retrieve a epoch change proof for SafetyRules so it can instantiate its
    /// ValidatorVerifier.
    fn retrieve_epoch_change_proof(&self, version: u64) -> Result<EpochChangeProof>;

    /// Returns a handle of the aptosdb.
    fn aptos_db(&self) -> Arc<dyn DbReader>;
}

#[derive(Clone)]
pub struct RootInfo(pub Block, pub QuorumCert, pub QuorumCert, pub QuorumCert);

/// LedgerRecoveryData is a subset of RecoveryData that we can get solely from ledger info.
#[derive(Clone)]
pub struct LedgerRecoveryData {
    storage_ledger: LedgerInfoWithSignatures,
}

impl LedgerRecoveryData {
    pub fn new(storage_ledger: LedgerInfoWithSignatures) -> Self {
        LedgerRecoveryData { storage_ledger }
    }

    /// Finds the root (last committed block) and returns the root block, the QC to the root block
    /// and the ledger info for the root block, return an error if it can not be found.
    ///
    /// We guarantee that the block corresponding to the storage's latest ledger info always exists.
    pub fn find_root(
        &self,
        blocks: &mut Vec<Block>,
        quorum_certs: &mut Vec<QuorumCert>,
    ) -> Result<RootInfo> {
        info!(
            "The last committed block id as recorded in storage: {}",
            self.storage_ledger
        );

        // We start from the block that storage's latest ledger info, if storage has end-epoch
        // LedgerInfo, we generate the virtual genesis block
        let (root_id, latest_ledger_info_sig) = if self.storage_ledger.ledger_info().ends_epoch() {
            let genesis =
                Block::make_genesis_block_from_ledger_info(self.storage_ledger.ledger_info());
            let genesis_qc = QuorumCert::certificate_for_genesis_from_ledger_info(
                self.storage_ledger.ledger_info(),
                genesis.id(),
            );
            let genesis_ledger_info = genesis_qc.ledger_info().clone();
            let genesis_id = genesis.id();
            blocks.push(genesis);
            quorum_certs.push(genesis_qc);
            (genesis_id, genesis_ledger_info)
        } else {
            (
                self.storage_ledger.ledger_info().consensus_block_id(),
                self.storage_ledger.clone(),
            )
        };

        // sort by (epoch, round) to guarantee the topological order of parent <- child
        blocks.sort_by_key(|b| (b.epoch(), b.round()));

        let root_idx = blocks
            .iter()
            .position(|block| block.id() == root_id)
            .ok_or_else(|| format_err!("unable to find root: {}", root_id))?;
        let root_block = blocks.remove(root_idx);
        let root_quorum_cert = quorum_certs
            .iter()
            .find(|qc| qc.certified_block().id() == root_block.id())
            .ok_or_else(|| format_err!("No QC found for root: {}", root_id))?
            .clone();
        let root_ordered_cert = quorum_certs
            .iter()
            .find(|qc| qc.commit_info().id() == root_block.id())
            .ok_or_else(|| format_err!("No LI found for root: {}", root_id))?
            .clone();

        info!("Consensus root block is {}", root_block);

        let root_commit_cert = root_ordered_cert
            .create_merged_with_executed_state(latest_ledger_info_sig)
            .expect("Inconsistent commit proof and evaluation decision, cannot commit block");

        Ok(RootInfo(
            root_block,
            root_quorum_cert,
            root_ordered_cert,
            root_commit_cert,
        ))
    }
}

pub struct RootMetadata {
    pub accu_hash: HashValue,
    pub frozen_root_hashes: Vec<HashValue>,
    pub num_leaves: Version,
}

impl RootMetadata {
    pub fn new(num_leaves: u64, accu_hash: HashValue, frozen_root_hashes: Vec<HashValue>) -> Self {
        Self {
            accu_hash,
            frozen_root_hashes,
            num_leaves,
        }
    }

    pub fn version(&self) -> Version {
        max(self.num_leaves, 1) - 1
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_empty() -> Self {
        Self::new(0, *aptos_crypto::hash::ACCUMULATOR_PLACEHOLDER_HASH, vec![])
    }
}

/// The recovery data constructed from raw consensusdb data, it'll find the root value and
/// blocks that need cleanup or return error if the input data is inconsistent.
pub struct RecoveryData {
    // The last vote message sent by this validator.
    last_vote: Option<Vote>,
    root: RootInfo,
    root_metadata: RootMetadata,
    // 1. the blocks guarantee the topological ordering - parent <- child.
    // 2. all blocks are children of the root.
    blocks: Vec<Block>,
    quorum_certs: Vec<QuorumCert>,
    blocks_to_prune: Option<Vec<HashValue>>,

    // Liveness data
    highest_2chain_timeout_certificate: Option<TwoChainTimeoutCertificate>,
}

impl RecoveryData {
    pub fn new(
        last_vote: Option<Vote>,
        ledger_recovery_data: LedgerRecoveryData,
        mut blocks: Vec<Block>,
        root_metadata: RootMetadata,
        mut quorum_certs: Vec<QuorumCert>,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
    ) -> Result<Self> {
        let root = ledger_recovery_data
            .find_root(&mut blocks, &mut quorum_certs)
            .with_context(|| {
                // for better readability
                quorum_certs.sort_by_key(|qc| qc.certified_block().round());
                format!(
                    "\nRoot id: {}\nBlocks in db: {}\nQuorum Certs in db: {}\n",
                    ledger_recovery_data
                        .storage_ledger
                        .ledger_info()
                        .consensus_block_id(),
                    blocks
                        .iter()
                        .map(|b| format!("\n\t{}", b))
                        .collect::<Vec<String>>()
                        .concat(),
                    quorum_certs
                        .iter()
                        .map(|qc| format!("\n\t{}", qc))
                        .collect::<Vec<String>>()
                        .concat(),
                )
            })?;

        let blocks_to_prune = Some(Self::find_blocks_to_prune(
            root.0.id(),
            &mut blocks,
            &mut quorum_certs,
        ));
        let epoch = root.0.epoch();
        Ok(RecoveryData {
            last_vote: match last_vote {
                Some(v) if v.epoch() == epoch => Some(v),
                _ => None,
            },
            root,
            root_metadata,
            blocks,
            quorum_certs,
            blocks_to_prune,
            highest_2chain_timeout_certificate: match highest_2chain_timeout_cert {
                Some(tc) if tc.epoch() == epoch => Some(tc),
                _ => None,
            },
        })
    }

    pub fn root_block(&self) -> &Block {
        &self.root.0
    }

    pub fn last_vote(&self) -> Option<Vote> {
        self.last_vote.clone()
    }

    pub fn take(self) -> (RootInfo, RootMetadata, Vec<Block>, Vec<QuorumCert>) {
        (
            self.root,
            self.root_metadata,
            self.blocks,
            self.quorum_certs,
        )
    }

    pub fn take_blocks_to_prune(&mut self) -> Vec<HashValue> {
        self.blocks_to_prune
            .take()
            .expect("blocks_to_prune already taken")
    }

    pub fn highest_2chain_timeout_certificate(&self) -> Option<TwoChainTimeoutCertificate> {
        self.highest_2chain_timeout_certificate.clone()
    }

    fn find_blocks_to_prune(
        root_id: HashValue,
        blocks: &mut Vec<Block>,
        quorum_certs: &mut Vec<QuorumCert>,
    ) -> Vec<HashValue> {
        // prune all the blocks that don't have root as ancestor
        let mut tree = HashSet::new();
        let mut to_remove = vec![];
        tree.insert(root_id);
        // assume blocks are sorted by round already
        blocks.retain(|block| {
            if tree.contains(&block.parent_id()) {
                tree.insert(block.id());
                true
            } else {
                to_remove.push(block.id());
                false
            }
        });
        quorum_certs.retain(|qc| tree.contains(&qc.certified_block().id()));
        to_remove
    }
}

/// The proxy we use to persist data in db storage service via grpc.
pub struct StorageWriteProxy {
    db: Arc<ConsensusDB>,
    aptos_db: Arc<dyn DbReader>,
}

impl StorageWriteProxy {
    pub fn new(config: &NodeConfig, aptos_db: Arc<dyn DbReader>) -> Self {
        let db = Arc::new(ConsensusDB::new(config.storage.dir()));
        StorageWriteProxy { db, aptos_db }
    }
}

impl PersistentLivenessStorage for StorageWriteProxy {
    fn save_tree(&self, blocks: Vec<Block>, quorum_certs: Vec<QuorumCert>) -> Result<()> {
        Ok(self
            .db
            .save_blocks_and_quorum_certificates(blocks, quorum_certs)?)
    }

    fn prune_tree(&self, block_ids: Vec<HashValue>) -> Result<()> {
        if !block_ids.is_empty() {
            // quorum certs that certified the block_ids will get removed
            self.db.delete_blocks_and_quorum_certificates(block_ids)?;
        }
        Ok(())
    }

    fn save_vote(&self, vote: &Vote) -> Result<()> {
        Ok(self.db.save_vote(bcs::to_bytes(vote)?)?)
    }

    fn recover_from_ledger(&self) -> LedgerRecoveryData {
        let startup_info = self
            .aptos_db
            .get_startup_info()
            .expect("unable to read ledger info from storage")
            .expect("startup info is None");

        LedgerRecoveryData::new(startup_info.latest_ledger_info)
    }

    fn start(&self) -> LivenessStorageData {
        info!("Start consensus recovery.");
        let raw_data = self
            .db
            .get_data()
            .expect("unable to recover consensus data");

        let last_vote = raw_data
            .0
            .map(|bytes| bcs::from_bytes(&bytes[..]).expect("unable to deserialize last vote"));

        let highest_2chain_timeout_cert = raw_data.1.map(|b| {
            bcs::from_bytes(&b).expect("unable to deserialize highest 2-chain timeout cert")
        });
        let blocks = raw_data.2;
        let quorum_certs: Vec<_> = raw_data.3;
        let blocks_repr: Vec<String> = blocks.iter().map(|b| format!("\n\t{}", b)).collect();
        info!(
            "The following blocks were restored from ConsensusDB : {}",
            blocks_repr.concat()
        );
        let qc_repr: Vec<String> = quorum_certs
            .iter()
            .map(|qc| format!("\n\t{}", qc))
            .collect();
        info!(
            "The following quorum certs were restored from ConsensusDB: {}",
            qc_repr.concat()
        );

        // find the block corresponding to storage latest ledger info
        let startup_info = self
            .aptos_db
            .get_startup_info()
            .expect("unable to read ledger info from storage")
            .expect("startup info is None");
        let ledger_recovery_data = LedgerRecoveryData::new(startup_info.latest_ledger_info.clone());
        let root_executed_trees = startup_info.committed_trees;

        match RecoveryData::new(
            last_vote,
            ledger_recovery_data.clone(),
            blocks,
            RootMetadata::new(
                root_executed_trees.txn_accumulator().num_leaves(),
                root_executed_trees.state_id(),
                root_executed_trees
                    .txn_accumulator()
                    .frozen_subtree_roots()
                    .clone(),
            ),
            quorum_certs,
            highest_2chain_timeout_cert,
        ) {
            Ok(mut initial_data) => {
                (self as &dyn PersistentLivenessStorage)
                    .prune_tree(initial_data.take_blocks_to_prune())
                    .expect("unable to prune dangling blocks during restart");
                if initial_data.last_vote.is_none() {
                    self.db
                        .delete_last_vote_msg()
                        .expect("unable to cleanup last vote");
                }
                if initial_data.highest_2chain_timeout_certificate.is_none() {
                    self.db
                        .delete_highest_2chain_timeout_certificate()
                        .expect("unable to cleanup highest 2-chain timeout cert");
                }
                info!(
                    "Starting up the consensus state machine with recovery data - [last_vote {}], [highest timeout certificate: {}]",
                    initial_data.last_vote.as_ref().map_or("None".to_string(), |v| v.to_string()),
                    initial_data.highest_2chain_timeout_certificate().as_ref().map_or("None".to_string(), |v| v.to_string()),
                );

                LivenessStorageData::RecoveryData(initial_data)
            }
            Err(e) => {
                error!(error = ?e, "Failed to construct recovery data");
                LivenessStorageData::LedgerRecoveryData(ledger_recovery_data)
            }
        }
    }

    fn save_highest_2chain_timeout_cert(
        &self,
        highest_timeout_cert: &TwoChainTimeoutCertificate,
    ) -> Result<()> {
        Ok(self
            .db
            .save_highest_2chain_timeout_certificate(bcs::to_bytes(highest_timeout_cert)?)?)
    }

    fn retrieve_epoch_change_proof(&self, version: u64) -> Result<EpochChangeProof> {
        let (_, proofs) = self
            .aptos_db
            .get_state_proof(version)
            .map_err(DbError::from)?
            .into_inner();
        Ok(proofs)
    }

    fn aptos_db(&self) -> Arc<dyn DbReader> {
        self.aptos_db.clone()
    }
}
