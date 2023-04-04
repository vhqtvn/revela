// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::ensure;
use aptos_consensus_types::proof_of_store::{BatchId, BatchInfo};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_types::{transaction::SignedTransaction, PeerId};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Clone, Eq, Deserialize, Serialize, PartialEq, Debug)]
pub struct PersistedValue {
    info: BatchInfo,
    maybe_payload: Option<Vec<SignedTransaction>>,
}

#[derive(PartialEq, Debug)]
pub(crate) enum StorageMode {
    PersistedOnly,
    MemoryAndPersisted,
}

impl PersistedValue {
    pub(crate) fn new(info: BatchInfo, maybe_payload: Option<Vec<SignedTransaction>>) -> Self {
        Self {
            info,
            maybe_payload,
        }
    }

    pub(crate) fn payload_storage_mode(&self) -> StorageMode {
        match self.maybe_payload {
            Some(_) => StorageMode::MemoryAndPersisted,
            None => StorageMode::PersistedOnly,
        }
    }

    pub(crate) fn take_payload(&mut self) -> Option<Vec<SignedTransaction>> {
        self.maybe_payload.take()
    }

    pub(crate) fn remove_payload(&mut self) {
        self.maybe_payload = None;
    }

    pub fn batch_info(&self) -> &BatchInfo {
        &self.info
    }
}

impl Deref for PersistedValue {
    type Target = BatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl TryFrom<PersistedValue> for Batch {
    type Error = anyhow::Error;

    fn try_from(value: PersistedValue) -> Result<Self, Self::Error> {
        Ok(Batch {
            batch_info: value.info,
            payload: BatchPayload::new(
                value
                    .maybe_payload
                    .ok_or_else(|| anyhow::anyhow!("Payload not exist"))?,
            ),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher)]
pub struct BatchPayload {
    txns: Vec<SignedTransaction>,
    #[serde(skip)]
    num_bytes: OnceCell<usize>,
}

impl CryptoHash for BatchPayload {
    type Hasher = BatchPayloadHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::new();
        let bytes = bcs::to_bytes(&self).expect("Unable to serialize batch payload");
        self.num_bytes.get_or_init(|| bytes.len());
        state.update(&bytes);
        state.finish()
    }
}

impl BatchPayload {
    pub fn new(txns: Vec<SignedTransaction>) -> Self {
        Self {
            txns,
            num_bytes: OnceCell::new(),
        }
    }

    pub fn into_transactions(self) -> Vec<SignedTransaction> {
        self.txns
    }

    pub fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn num_bytes(&self) -> usize {
        *self
            .num_bytes
            .get_or_init(|| bcs::serialized_size(&self).expect("unable to serialize batch payload"))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Batch {
    batch_info: BatchInfo,
    payload: BatchPayload,
}

impl Batch {
    pub fn new(
        batch_id: BatchId,
        payload: Vec<SignedTransaction>,
        epoch: u64,
        expiration: u64,
        batch_author: PeerId,
    ) -> Self {
        let payload = BatchPayload::new(payload);
        let batch_info = BatchInfo::new(
            batch_author,
            batch_id,
            epoch,
            expiration,
            payload.hash(),
            payload.num_txns() as u64,
            payload.num_bytes() as u64,
        );
        Self {
            batch_info,
            payload,
        }
    }

    pub fn verify(&self) -> anyhow::Result<()> {
        ensure!(
            self.payload.hash() == *self.digest(),
            "Payload hash doesn't match the digest"
        );
        ensure!(
            self.payload.num_txns() as u64 == self.num_txns(),
            "Payload num txns doesn't match batch info"
        );
        ensure!(
            self.payload.num_bytes() as u64 == self.num_bytes(),
            "Payload num bytes doesn't match batch info"
        );
        Ok(())
    }

    pub fn into_transactions(self) -> Vec<SignedTransaction> {
        self.payload.txns
    }

    pub fn batch_info(&self) -> &BatchInfo {
        &self.batch_info
    }
}

impl Deref for Batch {
    type Target = BatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.batch_info
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BatchRequest {
    epoch: u64,
    source: PeerId,
    digest: HashValue,
}

impl BatchRequest {
    pub fn new(source: PeerId, epoch: u64, digest: HashValue) -> Self {
        Self {
            epoch,
            source,
            digest,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        if self.source == peer_id {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Sender mismatch: peer_id: {}, source: {}",
                self.source,
                peer_id
            ))
        }
    }

    pub fn source(&self) -> PeerId {
        self.source
    }

    pub fn digest(&self) -> HashValue {
        self.digest
    }
}

impl From<Batch> for PersistedValue {
    fn from(value: Batch) -> Self {
        let Batch {
            batch_info,
            payload,
        } = value;
        PersistedValue::new(batch_info, Some(payload.into_transactions()))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchMsg {
    batch: Batch,
}

impl BatchMsg {
    pub fn new(batch: Batch) -> Self {
        Self { batch }
    }

    pub fn verify(&self, peer_id: PeerId) -> anyhow::Result<()> {
        ensure!(
            self.batch.author() == peer_id,
            "Batch author doesn't match sender"
        );
        self.batch.verify()
    }

    pub fn epoch(&self) -> u64 {
        self.batch.epoch()
    }

    pub fn unpack(self) -> Batch {
        self.batch
    }
}
