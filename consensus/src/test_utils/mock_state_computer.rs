// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError,
    payload_manager::PayloadManager,
    pipeline::buffer_manager::OrderedBlocks,
    state_computer::{PipelineExecutionResult, StateComputeResultFut},
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    transaction_deduper::TransactionDeduper,
    transaction_shuffler::TransactionShuffler,
};
use anyhow::Result;
use aptos_consensus_types::{block::Block, pipelined_block::PipelinedBlock};
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError, ExecutorResult, StateComputeResult};
use aptos_logger::debug;
use aptos_types::{
    block_executor::config::BlockExecutorConfigFromOnchain, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures, randomness::Randomness,
};
use futures::SinkExt;
use futures_channel::mpsc::UnboundedSender;
use std::sync::Arc;

pub struct EmptyStateComputer {
    executor_channel: UnboundedSender<OrderedBlocks>,
}

impl EmptyStateComputer {
    pub fn new(executor_channel: UnboundedSender<OrderedBlocks>) -> Self {
        Self { executor_channel }
    }
}

#[async_trait::async_trait]
impl StateComputer for EmptyStateComputer {
    async fn compute(
        &self,
        _block: &Block,
        _parent_block_id: HashValue,
        _randomness: Option<Randomness>,
    ) -> ExecutorResult<PipelineExecutionResult> {
        Ok(PipelineExecutionResult::new(
            vec![],
            StateComputeResult::new_dummy(),
        ))
    }

    async fn commit(
        &self,
        blocks: &[Arc<PipelinedBlock>],
        commit: LedgerInfoWithSignatures,
        call_back: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        assert!(!blocks.is_empty());

        if self
            .executor_channel
            .clone()
            .send(OrderedBlocks {
                ordered_blocks: blocks
                    .iter()
                    .map(|b| (**b).clone())
                    .collect::<Vec<PipelinedBlock>>(),
                ordered_proof: commit,
                callback: call_back,
            })
            .await
            .is_err()
        {
            debug!("Failed to send to buffer manager, maybe epoch ends");
        }

        Ok(())
    }

    async fn sync_to(&self, _commit: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _: Arc<PayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: BlockExecutorConfigFromOnchain,
        _: Arc<dyn TransactionDeduper>,
        _: bool,
    ) {
    }

    fn end_epoch(&self) {}
}

/// Random Compute Result State Computer
/// When compute(), if parent id is random_compute_result_root_hash, it returns Err(Error::BlockNotFound(parent_block_id))
/// Otherwise, it returns a dummy StateComputeResult with root hash as random_compute_result_root_hash.
pub struct RandomComputeResultStateComputer {
    random_compute_result_root_hash: HashValue,
}

impl RandomComputeResultStateComputer {
    pub fn new() -> Self {
        Self {
            random_compute_result_root_hash: HashValue::random(),
        }
    }

    pub fn get_root_hash(&self) -> HashValue {
        self.random_compute_result_root_hash
    }
}

#[async_trait::async_trait]
impl StateComputer for RandomComputeResultStateComputer {
    async fn schedule_compute(
        &self,
        _block: &Block,
        parent_block_id: HashValue,
        _randomness: Option<Randomness>,
    ) -> StateComputeResultFut {
        // trapdoor for Execution Error
        let res = if parent_block_id == self.random_compute_result_root_hash {
            Err(ExecutorError::BlockNotFound(parent_block_id))
        } else {
            Ok(StateComputeResult::new_dummy_with_root_hash(
                self.random_compute_result_root_hash,
            ))
        };
        let pipeline_execution_res = res.map(|res| PipelineExecutionResult::new(vec![], res));
        Box::pin(async move { pipeline_execution_res })
    }

    async fn commit(
        &self,
        _blocks: &[Arc<PipelinedBlock>],
        _commit: LedgerInfoWithSignatures,
        _call_back: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        Ok(())
    }

    async fn sync_to(&self, _commit: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _: Arc<PayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: BlockExecutorConfigFromOnchain,
        _: Arc<dyn TransactionDeduper>,
        _: bool,
    ) {
    }

    fn end_epoch(&self) {}
}
