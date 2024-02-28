// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::buffer_manager::OrderedBlocks,
    rand::rand_gen::types::{MockShare, RandShare},
};
use aptos_consensus_types::{
    block::Block,
    block_data::{BlockData, BlockType},
    common::{Author, Round},
    pipelined_block::PipelinedBlock,
    quorum_cert::QuorumCert,
};
use aptos_crypto::HashValue;
use aptos_executor_types::StateComputeResult;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    randomness::RandMetadata,
};

pub fn create_ordered_blocks(rounds: Vec<Round>) -> OrderedBlocks {
    let blocks = rounds
        .into_iter()
        .map(|round| {
            PipelinedBlock::new(
                Block::new_for_testing(
                    HashValue::random(),
                    BlockData::new_for_testing(
                        1,
                        round,
                        1,
                        QuorumCert::dummy(),
                        BlockType::Genesis,
                    ),
                    None,
                ),
                vec![],
                StateComputeResult::new_dummy(),
            )
        })
        .collect();
    OrderedBlocks {
        ordered_blocks: blocks,
        ordered_proof: LedgerInfoWithSignatures::new(
            LedgerInfo::mock_genesis(None),
            AggregateSignature::empty(),
        ),
        callback: Box::new(move |_, _| {}),
    }
}

pub(super) fn create_share_for_round(
    epoch: u64,
    round: Round,
    author: Author,
) -> RandShare<MockShare> {
    RandShare::<MockShare>::new(
        author,
        RandMetadata::new(epoch, round, HashValue::zero(), 1700000000),
        MockShare,
    )
}

pub(super) fn create_share(rand_metadata: RandMetadata, author: Author) -> RandShare<MockShare> {
    RandShare::<MockShare>::new(author, rand_metadata, MockShare)
}
