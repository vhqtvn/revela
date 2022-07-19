// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{proof_fetcher::ProofFetcher, DbReader};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use parking_lot::RwLock;
use std::collections::HashMap;

/// An implementation of proof fetcher, which synchronously fetches proofs from the underlying persistent
/// storage.
pub struct SyncProofFetcher<'a> {
    reader: &'a dyn DbReader,
    state_proof_cache: RwLock<HashMap<HashValue, SparseMerkleProof>>,
}

impl<'a> SyncProofFetcher<'a> {
    pub fn new(reader: &'a dyn DbReader) -> Self {
        Self {
            reader,
            state_proof_cache: RwLock::new(HashMap::new()),
        }
    }
}

impl<'a> ProofFetcher for SyncProofFetcher<'a> {
    fn fetch_state_value_and_proof(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> anyhow::Result<(Option<StateValue>, Option<SparseMerkleProof>)> {
        let (state_value, proof) = self
            .reader
            .get_state_value_with_proof_by_version(state_key, version)?;
        // multiple threads may enter this code, and another thread might add
        // an address before this one. Thus the insertion might return a None here.
        self.state_proof_cache
            .write()
            .insert(state_key.hash(), proof.clone());

        Ok((state_value, Some(proof)))
    }

    fn get_proof_cache(&self) -> HashMap<HashValue, SparseMerkleProof> {
        self.state_proof_cache
            .read()
            .iter()
            .map(|(x, y)| (*x, y.clone()))
            .collect()
    }
}
