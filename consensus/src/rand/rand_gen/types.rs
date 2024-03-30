// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::bls12381::Signature;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::{
    pvss::{Player, WeightedConfig},
    weighted_vuf::traits::WeightedVUF,
};
use aptos_logger::debug;
use aptos_runtimes::spawn_rayon_thread_pool;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    randomness::{
        Delta, PKShare, ProofShare, RandKeys, RandMetadata, Randomness, WvufPP, APK, WVUF,
    },
    validator_verifier::ValidatorVerifier,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{fmt::Debug, sync::Arc};

pub const NUM_THREADS_FOR_WVUF_DERIVATION: usize = 8;
pub const FUTURE_ROUNDS_TO_ACCEPT: u64 = 200;

#[derive(PartialEq)]
pub enum PathType {
    Fast,
    Slow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockShare;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockAugData;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Share {
    share: ProofShare,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AugmentedData {
    delta: Delta,
    fast_delta: Option<Delta>,
}

impl TShare for Share {
    fn verify(
        &self,
        rand_config: &RandConfig,
        rand_metadata: &RandMetadata,
        author: &Author,
    ) -> anyhow::Result<()> {
        let index = *rand_config
            .validator
            .address_to_validator_index()
            .get(author)
            .unwrap();
        let maybe_apk = &rand_config.keys.certified_apks[index];
        if let Some(apk) = maybe_apk.get() {
            WVUF::verify_share(
                &rand_config.vuf_pp,
                apk,
                rand_metadata.to_bytes().as_slice(),
                &self.share,
            )?;
        } else {
            bail!(
                "[RandShare] No augmented public key for validator id {}, {}",
                index,
                author
            );
        }
        Ok(())
    }

    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized,
    {
        let share = Share {
            share: WVUF::create_share(&rand_config.keys.ask, rand_metadata.to_bytes().as_slice()),
        };
        RandShare::new(rand_config.author(), rand_metadata, share)
    }

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Randomness
    where
        Self: Sized,
    {
        let timer = std::time::Instant::now();
        let mut apks_and_proofs = vec![];
        for share in shares {
            let id = *rand_config
                .validator
                .address_to_validator_index()
                .get(share.author())
                .unwrap();
            let apk = rand_config.get_certified_apk(share.author()).unwrap(); // needs to have apk to verify the share
            apks_and_proofs.push((Player { id }, apk.clone(), share.share().share));
        }

        let proof = WVUF::aggregate_shares(&rand_config.wconfig, &apks_and_proofs);
        let pool =
            spawn_rayon_thread_pool("wvuf".to_string(), Some(NUM_THREADS_FOR_WVUF_DERIVATION));
        let eval = WVUF::derive_eval(
            &rand_config.wconfig,
            &rand_config.vuf_pp,
            rand_metadata.to_bytes().as_slice(),
            &rand_config.get_all_certified_apk(),
            &proof,
            &pool,
        )
        .expect("All APK should exist");
        debug!(
            "WVUF derivation time: {} ms, number of threads: {}",
            timer.elapsed().as_millis(),
            NUM_THREADS_FOR_WVUF_DERIVATION
        );
        let eval_bytes = bcs::to_bytes(&eval).unwrap();
        let rand_bytes = Sha3_256::digest(eval_bytes.as_slice()).to_vec();
        Randomness::new(rand_metadata.clone(), rand_bytes)
    }
}

impl TAugmentedData for AugmentedData {
    fn generate(rand_config: &RandConfig, fast_rand_config: &Option<RandConfig>) -> AugData<Self>
    where
        Self: Sized,
    {
        let delta = rand_config.get_my_delta().clone();
        rand_config
            .add_certified_delta(&rand_config.author(), delta.clone())
            .expect("Add self delta should succeed");

        let fast_delta = if let Some(fast_config) = fast_rand_config.as_ref() {
            let fast_delta = fast_config.get_my_delta().clone();
            fast_config
                .add_certified_delta(&rand_config.author(), fast_delta.clone())
                .expect("Add self delta for fast path should succeed");
            Some(fast_delta)
        } else {
            None
        };

        let data = AugmentedData {
            delta: delta.clone(),
            fast_delta,
        };
        AugData::new(rand_config.epoch(), rand_config.author(), data)
    }

    fn augment(
        &self,
        rand_config: &RandConfig,
        fast_rand_config: &Option<RandConfig>,
        author: &Author,
    ) {
        let AugmentedData { delta, fast_delta } = self;
        rand_config
            .add_certified_delta(author, delta.clone())
            .expect("Add delta should succeed");

        if let (Some(config), Some(fast_delta)) = (fast_rand_config, fast_delta) {
            config
                .add_certified_delta(author, fast_delta.clone())
                .expect("Add delta for fast path should succeed");
        }
    }

    fn verify(
        &self,
        rand_config: &RandConfig,
        fast_rand_config: &Option<RandConfig>,
        author: &Author,
    ) -> anyhow::Result<()> {
        rand_config
            .derive_apk(author, self.delta.clone())
            .map(|_| ())?;

        ensure!(
            self.fast_delta.is_some() == fast_rand_config.is_some(),
            "Fast path delta should be present iff fast_rand_config is present."
        );
        if let (Some(config), Some(fast_delta)) = (fast_rand_config, self.fast_delta.as_ref()) {
            config.derive_apk(author, fast_delta.clone()).map(|_| ())
        } else {
            Ok(())
        }
    }
}

impl TShare for MockShare {
    fn verify(
        &self,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
        _author: &Author,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized,
    {
        RandShare::new(rand_config.author(), rand_metadata, Self)
    }

    fn aggregate<'a>(
        _shares: impl Iterator<Item = &'a RandShare<Self>>,
        _rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Randomness
    where
        Self: Sized,
    {
        Randomness::new(rand_metadata, vec![])
    }
}

impl TAugmentedData for MockAugData {
    fn generate(rand_config: &RandConfig, _fast_rand_config: &Option<RandConfig>) -> AugData<Self>
    where
        Self: Sized,
    {
        AugData::new(rand_config.epoch(), rand_config.author(), Self)
    }

    fn augment(
        &self,
        _rand_config: &RandConfig,
        _fast_rand_config: &Option<RandConfig>,
        _author: &Author,
    ) {
    }

    fn verify(
        &self,
        _rand_config: &RandConfig,
        _fast_rand_config: &Option<RandConfig>,
        _author: &Author,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait TShare:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn verify(
        &self,
        rand_config: &RandConfig,
        rand_metadata: &RandMetadata,
        author: &Author,
    ) -> anyhow::Result<()>;

    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized;

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Randomness
    where
        Self: Sized;
}

pub trait TAugmentedData:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn generate(rand_config: &RandConfig, fast_rand_config: &Option<RandConfig>) -> AugData<Self>
    where
        Self: Sized;

    fn augment(
        &self,
        rand_config: &RandConfig,
        fast_rand_config: &Option<RandConfig>,
        author: &Author,
    );

    fn verify(
        &self,
        rand_config: &RandConfig,
        fast_rand_config: &Option<RandConfig>,
        author: &Author,
    ) -> anyhow::Result<()>;
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ShareId {
    epoch: u64,
    round: Round,
    author: Author,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RandShare<S> {
    author: Author,
    metadata: RandMetadata,
    share: S,
}

impl<S: TShare> RandShare<S> {
    pub fn new(author: Author, metadata: RandMetadata, share: S) -> Self {
        Self {
            author,
            metadata,
            share,
        }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn share(&self) -> &S {
        &self.share
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch()
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.share.verify(rand_config, &self.metadata, &self.author)
    }

    pub fn share_id(&self) -> ShareId {
        ShareId {
            epoch: self.epoch(),
            round: self.round(),
            author: self.author,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RequestShare {
    epoch: u64,
    rand_metadata: RandMetadata,
}

impl RequestShare {
    pub fn new(epoch: u64, rand_metadata: RandMetadata) -> Self {
        Self {
            epoch,
            rand_metadata,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn rand_metadata(&self) -> &RandMetadata {
        &self.rand_metadata
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FastShare<S> {
    pub share: RandShare<S>,
}

impl<S: TShare> FastShare<S> {
    pub fn new(share: RandShare<S>) -> Self {
        Self { share }
    }

    pub fn author(&self) -> &Author {
        self.share.author()
    }

    pub fn rand_share(&self) -> RandShare<S> {
        self.share.clone()
    }

    pub fn share(&self) -> &S {
        self.share.share()
    }

    pub fn metadata(&self) -> &RandMetadata {
        self.share.metadata()
    }

    pub fn round(&self) -> Round {
        self.share.round()
    }

    pub fn epoch(&self) -> u64 {
        self.share.epoch()
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.share.verify(rand_config)
    }

    pub fn share_id(&self) -> ShareId {
        self.share.share_id()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct AugDataId {
    epoch: u64,
    author: Author,
}

impl AugDataId {
    pub fn new(epoch: u64, author: Author) -> Self {
        Self { epoch, author }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn author(&self) -> Author {
        self.author
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AugData<D> {
    epoch: u64,
    author: Author,
    data: D,
}

impl<D: TAugmentedData> AugData<D> {
    pub fn new(epoch: u64, author: Author, data: D) -> Self {
        Self {
            epoch,
            author,
            data,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn id(&self) -> AugDataId {
        AugDataId {
            epoch: self.epoch,
            author: self.author,
        }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn verify(
        &self,
        rand_config: &RandConfig,
        fast_rand_config: &Option<RandConfig>,
        sender: Author,
    ) -> anyhow::Result<()> {
        ensure!(self.author == sender, "Invalid author");
        self.data
            .verify(rand_config, fast_rand_config, &self.author)?;
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AugDataSignature {
    epoch: u64,
    signature: Signature,
}

impl AugDataSignature {
    pub fn new(epoch: u64, signature: Signature) -> Self {
        Self { epoch, signature }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn verify<D: TAugmentedData>(
        &self,
        author: Author,
        verifier: &ValidatorVerifier,
        data: &AugData<D>,
    ) -> anyhow::Result<()> {
        Ok(verifier.verify(author, data, &self.signature)?)
    }

    pub fn into_signature(self) -> Signature {
        self.signature
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CertifiedAugData<D> {
    aug_data: AugData<D>,
    signatures: AggregateSignature,
}

impl<D: TAugmentedData> CertifiedAugData<D> {
    pub fn new(aug_data: AugData<D>, signatures: AggregateSignature) -> Self {
        Self {
            aug_data,
            signatures,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.aug_data.epoch()
    }

    pub fn id(&self) -> AugDataId {
        self.aug_data.id()
    }

    pub fn author(&self) -> &Author {
        self.aug_data.author()
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        verifier.verify_multi_signatures(&self.aug_data, &self.signatures)?;
        Ok(())
    }

    pub fn data(&self) -> &D {
        &self.aug_data.data
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CertifiedAugDataAck {
    epoch: u64,
}

impl CertifiedAugDataAck {
    pub fn new(epoch: u64) -> Self {
        Self { epoch }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[derive(Clone)]
pub struct RandConfig {
    author: Author,
    epoch: u64,
    validator: ValidatorVerifier,
    // public parameters of the weighted VUF
    vuf_pp: WvufPP,
    // key shares for weighted VUF
    keys: Arc<RandKeys>,
    // weighted config for weighted VUF
    wconfig: WeightedConfig,
}

impl Debug for RandConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RandConfig {{ epoch: {}, author: {}, wconfig: {:?} }}",
            self.epoch, self.author, self.wconfig
        )
    }
}

impl RandConfig {
    pub fn new(
        author: Author,
        epoch: u64,
        validator: ValidatorVerifier,
        vuf_pp: WvufPP,
        keys: RandKeys,
        wconfig: WeightedConfig,
    ) -> Self {
        Self {
            author,
            epoch,
            validator,
            vuf_pp,
            keys: Arc::new(keys),
            wconfig,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn get_id(&self, peer: &Author) -> usize {
        *self
            .validator
            .address_to_validator_index()
            .get(peer)
            .unwrap()
    }

    pub fn get_certified_apk(&self, peer: &Author) -> Option<&APK> {
        let index = self.get_id(peer);
        self.keys.certified_apks[index].get()
    }

    pub fn get_all_certified_apk(&self) -> Vec<Option<APK>> {
        self.keys
            .certified_apks
            .iter()
            .map(|cell| cell.get().cloned())
            .collect()
    }

    pub fn add_certified_apk(&self, peer: &Author, apk: APK) -> anyhow::Result<()> {
        let index = self.get_id(peer);
        self.keys.add_certified_apk(index, apk)
    }

    fn derive_apk(&self, peer: &Author, delta: Delta) -> anyhow::Result<APK> {
        let apk = WVUF::augment_pubkey(&self.vuf_pp, self.get_pk_share(peer).clone(), delta)?;
        Ok(apk)
    }

    pub fn add_certified_delta(&self, peer: &Author, delta: Delta) -> anyhow::Result<()> {
        let apk = self.derive_apk(peer, delta)?;
        self.add_certified_apk(peer, apk)?;
        Ok(())
    }

    pub fn get_my_delta(&self) -> &Delta {
        WVUF::get_public_delta(&self.keys.apk)
    }

    pub fn get_pk_share(&self, peer: &Author) -> &PKShare {
        let index = self.get_id(peer);
        &self.keys.pk_shares[index]
    }

    pub fn get_peer_weight(&self, peer: &Author) -> u64 {
        let player = Player {
            id: self.get_id(peer),
        };
        self.wconfig.get_player_weight(&player) as u64
    }

    pub fn threshold(&self) -> u64 {
        self.wconfig.get_threshold_weight() as u64
    }
}
