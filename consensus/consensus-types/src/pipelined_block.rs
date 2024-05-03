// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::Block,
    common::{Payload, Round},
    quorum_cert::QuorumCert,
    vote_proposal::VoteProposal,
};
use aptos_crypto::hash::HashValue;
use aptos_executor_types::StateComputeResult;
use aptos_types::{
    block_info::BlockInfo, contract_event::ContractEvent, randomness::Randomness,
    transaction::SignedTransaction, validator_txn::ValidatorTransaction,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{Debug, Display, Formatter},
    time::{Duration, Instant},
};

/// A representation of a block that has been added to the execution pipeline. It might either be in ordered
/// or in executed state. In the ordered state, the block is waiting to be executed. In the executed state,
/// the block has been executed and the output is available.
#[derive(Clone, Eq, PartialEq)]
pub struct PipelinedBlock {
    /// Block data that cannot be regenerated.
    block: Block,
    /// Input transactions in the order of execution
    input_transactions: Vec<SignedTransaction>,
    /// The state_compute_result is calculated for all the pending blocks prior to insertion to
    /// the tree. The execution results are not persisted: they're recalculated again for the
    /// pending blocks upon restart.
    state_compute_result: StateComputeResult,
    randomness: OnceCell<Randomness>,
    pipeline_insertion_time: OnceCell<Instant>,
}

impl Serialize for PipelinedBlock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename = "PipelineBlock")]
        struct SerializedBlock<'a> {
            block: &'a Block,
            input_transactions: &'a Vec<SignedTransaction>,
            state_compute_result: &'a StateComputeResult,
            randomness: Option<&'a Randomness>,
        }

        let serialized = SerializedBlock {
            block: &self.block,
            input_transactions: &self.input_transactions,
            state_compute_result: &self.state_compute_result,
            randomness: self.randomness.get(),
        };
        serialized.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PipelinedBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "PipelineBlock")]
        struct SerializedBlock {
            block: Block,
            input_transactions: Vec<SignedTransaction>,
            state_compute_result: StateComputeResult,
            randomness: Option<Randomness>,
        }

        let SerializedBlock {
            block,
            input_transactions,
            state_compute_result,
            randomness,
        } = SerializedBlock::deserialize(deserializer)?;

        let block = PipelinedBlock {
            block,
            input_transactions,
            state_compute_result,
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
        };
        if let Some(r) = randomness {
            block.set_randomness(r);
        }
        Ok(block)
    }
}

impl PipelinedBlock {
    pub fn set_execution_result(
        mut self,
        input_transactions: Vec<SignedTransaction>,
        result: StateComputeResult,
    ) -> Self {
        self.state_compute_result = result;
        self.input_transactions = input_transactions;
        self
    }

    pub fn set_randomness(&self, randomness: Randomness) {
        assert!(self.randomness.set(randomness).is_ok());
    }

    pub fn set_insertion_time(&self) {
        assert!(self.pipeline_insertion_time.set(Instant::now()).is_ok());
    }
}

impl Debug for PipelinedBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for PipelinedBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.block())
    }
}

impl PipelinedBlock {
    pub fn new(
        block: Block,
        input_transactions: Vec<SignedTransaction>,
        state_compute_result: StateComputeResult,
    ) -> Self {
        Self {
            block,
            input_transactions,
            state_compute_result,
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
        }
    }

    pub fn new_ordered(block: Block) -> Self {
        Self {
            block,
            input_transactions: vec![],
            state_compute_result: StateComputeResult::new_dummy(),
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn id(&self) -> HashValue {
        self.block().id()
    }

    pub fn input_transactions(&self) -> &Vec<SignedTransaction> {
        &self.input_transactions
    }

    pub fn epoch(&self) -> u64 {
        self.block.epoch()
    }

    pub fn payload(&self) -> Option<&Payload> {
        self.block().payload()
    }

    pub fn parent_id(&self) -> HashValue {
        self.block.parent_id()
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        self.block().quorum_cert()
    }

    pub fn round(&self) -> Round {
        self.block().round()
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        self.block().validator_txns()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.block().timestamp_usecs()
    }

    pub fn compute_result(&self) -> &StateComputeResult {
        &self.state_compute_result
    }

    pub fn randomness(&self) -> Option<&Randomness> {
        self.randomness.get()
    }

    pub fn has_randomness(&self) -> bool {
        self.randomness.get().is_some()
    }

    pub fn block_info(&self) -> BlockInfo {
        self.block().gen_block_info(
            self.compute_result().root_hash(),
            self.compute_result().version(),
            self.compute_result().epoch_state().clone(),
        )
    }

    pub fn vote_proposal(&self) -> VoteProposal {
        VoteProposal::new(
            self.compute_result().extension_proof(),
            self.block.clone(),
            self.compute_result().epoch_state().clone(),
            true,
        )
    }

    pub fn subscribable_events(&self) -> Vec<ContractEvent> {
        // reconfiguration suffix don't count, the state compute result is carried over from parents
        if self.is_reconfiguration_suffix() {
            return vec![];
        }
        self.state_compute_result.subscribable_events().to_vec()
    }

    /// The block is suffix of a reconfiguration block if the state result carries over the epoch state
    /// from parent but has no transaction.
    pub fn is_reconfiguration_suffix(&self) -> bool {
        self.state_compute_result.has_reconfiguration()
            && self
                .state_compute_result
                .compute_status_for_input_txns()
                .is_empty()
    }

    pub fn elapsed_in_pipeline(&self) -> Option<Duration> {
        self.pipeline_insertion_time.get().map(|t| t.elapsed())
    }
}
