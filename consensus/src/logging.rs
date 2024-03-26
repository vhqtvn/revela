// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use aptos_crypto::HashValue;
use aptos_logger::Schema;
use aptos_types::block_info::Round;
use serde::Serialize;

#[derive(Schema)]
pub struct LogSchema {
    event: LogEvent,
    author: Option<Author>,
    remote_peer: Option<Author>,
    epoch: Option<u64>,
    round: Option<Round>,
    id: Option<HashValue>,
}

#[derive(Serialize)]
pub enum LogEvent {
    CommitViaBlock,
    CommitViaSync,
    IncrementalProofExpired,
    NetworkReceiveProposal,
    NewEpoch,
    NewRound,
    ProofOfStoreInit,
    ProofOfStoreReady,
    Propose,
    ReceiveBatchRetrieval,
    ReceiveBlockRetrieval,
    ReceiveEpochChangeProof,
    ReceiveEpochRetrieval,
    ReceiveMessageFromDifferentEpoch,
    ReceiveNewCertificate,
    ReceiveProposal,
    ReceiveSyncInfo,
    ReceiveVote,
    RetrieveBlock,
    StateSync,
    Timeout,
    Vote,
    VoteNIL,
    // log events related to randomness generation
    BroadcastRandShare,
    ReceiveProactiveRandShare,
    ReceiveReactiveRandShare,
    BroadcastAugData,
    ReceiveAugData,
    BroadcastCertifiedAugData,
    ReceiveCertifiedAugData,
    // randomness fast path
    BroadcastRandShareFastPath,
    ReceiveRandShareFastPath,
}

impl LogSchema {
    pub fn new(event: LogEvent) -> Self {
        Self {
            event,
            author: None,
            remote_peer: None,
            epoch: None,
            round: None,
            id: None,
        }
    }
}
