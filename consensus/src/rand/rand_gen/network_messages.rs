// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
    rand::rand_gen::types::{
        AugData, AugDataSignature, AugmentedData, CertifiedAugData, CertifiedAugDataAck, Proof,
        RandConfig, RandShare, Share, ShareAck,
    },
};
use anyhow::bail;
use aptos_consensus_types::common::Author;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::RBMessage;
use aptos_types::epoch_state::EpochState;
use bytes::Bytes;
use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use std::cmp::min;

#[derive(Clone, Serialize, Deserialize, EnumConversion)]
pub enum RandMessage<S, P, D> {
    Share(RandShare<S>),
    ShareAck(ShareAck<P>),
    AugData(AugData<D>),
    AugDataSignature(AugDataSignature),
    CertifiedAugData(CertifiedAugData<D>),
    CertifiedAugDataAck(CertifiedAugDataAck),
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> RandMessage<S, P, D> {
    pub fn verify(
        &self,
        epoch_state: &EpochState,
        rand_config: &RandConfig,
        sender: Author,
    ) -> anyhow::Result<()> {
        match self {
            RandMessage::Share(share) => share.verify(rand_config),
            RandMessage::AugData(aug_data) => aug_data.verify(sender),
            RandMessage::CertifiedAugData(certified_aug_data) => {
                certified_aug_data.verify(&epoch_state.verifier)
            },
            _ => bail!("[RandMessage] unexpected message type"),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> RBMessage for RandMessage<S, P, D> {}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> TConsensusMsg for RandMessage<S, P, D> {
    fn epoch(&self) -> u64 {
        match self {
            RandMessage::Share(share) => share.epoch(),
            RandMessage::ShareAck(ack) => ack.epoch(),
            RandMessage::AugData(aug_data) => aug_data.epoch(),
            RandMessage::AugDataSignature(signature) => signature.epoch(),
            RandMessage::CertifiedAugData(certified_aug_data) => certified_aug_data.epoch(),
            RandMessage::CertifiedAugDataAck(ack) => ack.epoch(),
        }
    }

    fn from_network_message(msg: ConsensusMsg) -> anyhow::Result<Self> {
        match msg {
            ConsensusMsg::RandGenMessage(msg) => Ok(bcs::from_bytes(&msg.data)?),
            _ => bail!("unexpected consensus message type {:?}", msg),
        }
    }

    fn into_network_message(self) -> ConsensusMsg {
        ConsensusMsg::RandGenMessage(RandGenMessage {
            epoch: self.epoch(),
            data: bcs::to_bytes(&self).unwrap(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RandGenMessage {
    epoch: u64,
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

impl RandGenMessage {
    pub fn new(epoch: u64, data: Vec<u8>) -> Self {
        Self { epoch, data }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl core::fmt::Debug for RandGenMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RandGenMessage")
            .field("epoch", &self.epoch)
            .field("data", &hex::encode(&self.data[..min(20, self.data.len())]))
            .finish()
    }
}

pub struct RpcRequest<S, P, D> {
    pub req: RandMessage<S, P, D>,
    pub protocol: ProtocolId,
    pub response_sender: oneshot::Sender<Result<Bytes, RpcError>>,
}
