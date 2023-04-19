// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::proof_manager::ProofManager;
use aptos_consensus_types::{
    common::{Payload, PayloadFilter},
    proof_of_store::{BatchId, BatchInfo, ProofOfStore},
    request_response::{GetPayloadCommand, GetPayloadResponse},
};
use aptos_crypto::HashValue;
use aptos_types::{aggregate_signature::AggregateSignature, PeerId};
use futures::channel::oneshot;
use std::collections::HashSet;

fn create_proof_manager() -> ProofManager {
    ProofManager::new(PeerId::random(), 10, 10)
}

fn create_proof(author: PeerId, expiration: u64, batch_sequence: u64) -> ProofOfStore {
    create_proof_with_gas(author, expiration, batch_sequence, 0)
}

fn create_proof_with_gas(
    author: PeerId,
    expiration: u64,
    batch_sequence: u64,
    gas_bucket_start: u64,
) -> ProofOfStore {
    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(batch_sequence);
    ProofOfStore::new(
        BatchInfo::new(
            author,
            batch_id,
            0,
            expiration,
            digest,
            1,
            1,
            gas_bucket_start,
        ),
        AggregateSignature::empty(),
    )
}

async fn get_proposal_and_assert(
    proof_manager: &mut ProofManager,
    max_txns: u64,
    filter: &[BatchInfo],
    expected: &[ProofOfStore],
) {
    let (callback_tx, callback_rx) = oneshot::channel();
    let filter_set = HashSet::from_iter(filter.iter().cloned());
    let req = GetPayloadCommand::GetPayloadRequest(
        max_txns,
        1000000,
        true,
        PayloadFilter::InQuorumStore(filter_set),
        callback_tx,
    );
    proof_manager.handle_proposal_request(req);
    let GetPayloadResponse::GetPayloadResponse(payload) = callback_rx.await.unwrap().unwrap();
    if let Payload::InQuorumStore(proofs) = payload {
        assert_eq!(proofs.proofs.len(), expected.len());
        for proof in proofs.proofs {
            assert!(expected.contains(&proof));
        }
    } else {
        panic!("Unexpected variant")
    }
}

#[tokio::test]
async fn test_block_request() {
    let mut proof_manager = create_proof_manager();

    let proof = create_proof(PeerId::random(), 10, 1);
    proof_manager.receive_proofs(vec![proof.clone()]);

    get_proposal_and_assert(&mut proof_manager, 100, &[], &vec![proof.clone()]).await;
}

#[tokio::test]
async fn test_block_timestamp_expiration() {
    let mut proof_manager = create_proof_manager();

    let proof = create_proof(PeerId::random(), 10, 1);
    proof_manager.receive_proofs(vec![proof.clone()]);

    proof_manager.handle_commit_notification(1, vec![]);
    get_proposal_and_assert(&mut proof_manager, 100, &[], &vec![proof]).await;

    proof_manager.handle_commit_notification(20, vec![]);
    get_proposal_and_assert(&mut proof_manager, 100, &[], &[]).await;
}

#[tokio::test]
async fn test_batch_commit() {
    let mut proof_manager = create_proof_manager();

    let proof0 = create_proof(PeerId::random(), 10, 1);
    proof_manager.receive_proofs(vec![proof0.clone()]);

    let proof1 = create_proof(PeerId::random(), 11, 2);
    proof_manager.receive_proofs(vec![proof1.clone()]);

    proof_manager.handle_commit_notification(1, vec![proof1.info().clone()]);
    get_proposal_and_assert(&mut proof_manager, 100, &[], &vec![proof0]).await;
}

#[tokio::test]
async fn test_proposal_priority() {
    let mut proof_manager = create_proof_manager();
    let peer0 = PeerId::random();

    let peer0_proof0 = create_proof_with_gas(peer0, 10, 2, 1000);
    let peer0_proof1 = create_proof_with_gas(peer0, 10, 1, 0);
    proof_manager.receive_proofs(vec![peer0_proof1.clone(), peer0_proof0.clone()]);

    let peer0_proof2 = create_proof_with_gas(peer0, 10, 4, 500);
    proof_manager.receive_proofs(vec![peer0_proof2.clone()]);
    let peer0_proof3 = create_proof_with_gas(peer0, 10, 3, 500);
    proof_manager.receive_proofs(vec![peer0_proof3.clone()]);

    // Gas bucket is the most significant prioritization
    let expected = vec![peer0_proof0.clone()];
    get_proposal_and_assert(&mut proof_manager, 1, &[], &expected).await;

    // Batch sequence is prioritized next
    let expected = vec![peer0_proof3.clone()];
    get_proposal_and_assert(
        &mut proof_manager,
        1,
        &[peer0_proof0.info().clone()],
        &expected,
    )
    .await;
}

#[tokio::test]
async fn test_proposal_fairness() {
    let mut proof_manager = create_proof_manager();
    let peer0 = PeerId::random();
    let peer1 = PeerId::random();

    let mut peer0_proofs = vec![];
    for i in 0..4 {
        let proof = create_proof(peer0, 10 + i, 1 + i);
        proof_manager.receive_proofs(vec![proof.clone()]);
        peer0_proofs.push(proof);
    }

    let peer1_proof_0 = create_proof(peer1, 7, 1);
    proof_manager.receive_proofs(vec![peer1_proof_0.clone()]);

    // Without filter, and large max size, all proofs are retrieved
    let mut expected = peer0_proofs.clone();
    expected.push(peer1_proof_0.clone());
    get_proposal_and_assert(&mut proof_manager, 100, &[], &expected).await;

    // The first two proofs are taken fairly from each peer
    get_proposal_and_assert(&mut proof_manager, 2, &[], &vec![
        peer0_proofs[0].clone(),
        peer1_proof_0.clone(),
    ])
    .await;

    // The next two proofs are taken from the remaining peer
    let filter = vec![peer0_proofs[0].clone(), peer1_proof_0.clone()];
    let filter: Vec<_> = filter.iter().map(ProofOfStore::info).cloned().collect();
    get_proposal_and_assert(&mut proof_manager, 2, &filter, &peer0_proofs[1..3]).await;

    // The last proof is also taken from the remaining peer
    let mut filter = peer0_proofs[0..3].to_vec();
    filter.push(peer1_proof_0.clone());
    let filter: Vec<_> = filter.iter().map(ProofOfStore::info).cloned().collect();
    get_proposal_and_assert(&mut proof_manager, 2, &filter, &peer0_proofs[3..4]).await;
}

#[tokio::test]
async fn test_duplicate_batches_on_commit() {
    let mut proof_manager = create_proof_manager();

    let author = PeerId::random();
    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(1);
    let batch = BatchInfo::new(author, batch_id, 0, 10, digest, 1, 1, 0);
    let proof0 = ProofOfStore::new(batch.clone(), AggregateSignature::empty());
    let proof1 = ProofOfStore::new(batch.clone(), AggregateSignature::empty());
    let proof2 = ProofOfStore::new(batch.clone(), AggregateSignature::empty());

    proof_manager.receive_proofs(vec![proof0.clone()]);
    proof_manager.receive_proofs(vec![proof1.clone()]);

    // Only one copy of the batch exists
    get_proposal_and_assert(&mut proof_manager, 10, &[], &vec![proof0.clone()]).await;

    // Nothing goes wrong on commits
    proof_manager.handle_commit_notification(4, vec![batch.clone()]);
    get_proposal_and_assert(&mut proof_manager, 10, &[], &[]).await;

    // Before expiration, still marked as committed
    proof_manager.receive_proofs(vec![proof2.clone()]);
    get_proposal_and_assert(&mut proof_manager, 10, &[], &[]).await;

    // Nothing goes wrong on expiration
    proof_manager.handle_commit_notification(5, vec![]);
    get_proposal_and_assert(&mut proof_manager, 10, &[], &[]).await;
    proof_manager.handle_commit_notification(12, vec![]);
    get_proposal_and_assert(&mut proof_manager, 10, &[], &[]).await;
}

#[tokio::test]
async fn test_duplicate_batches_on_expiration() {
    let mut proof_manager = create_proof_manager();

    let author = PeerId::random();
    let digest = HashValue::random();
    let batch_id = BatchId::new_for_test(1);
    let batch = BatchInfo::new(author, batch_id, 0, 10, digest, 1, 1, 0);
    let proof0 = ProofOfStore::new(batch.clone(), AggregateSignature::empty());
    let proof1 = ProofOfStore::new(batch.clone(), AggregateSignature::empty());

    proof_manager.receive_proofs(vec![proof0.clone()]);
    proof_manager.receive_proofs(vec![proof1.clone()]);

    // Only one copy of the batch exists
    get_proposal_and_assert(&mut proof_manager, 10, &[], &vec![proof0.clone()]).await;

    // Nothing goes wrong on expiration
    proof_manager.handle_commit_notification(5, vec![]);
    get_proposal_and_assert(&mut proof_manager, 10, &[], &vec![proof0.clone()]).await;
    proof_manager.handle_commit_notification(12, vec![]);
    get_proposal_and_assert(&mut proof_manager, 10, &[], &[]).await;
}
