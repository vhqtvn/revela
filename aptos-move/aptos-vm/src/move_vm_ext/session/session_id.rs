// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_metadata::TransactionMetadata;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    block_metadata::BlockMetadata, block_metadata_ext::BlockMetadataExt,
    validator_txn::ValidatorTransaction,
};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(BCSCryptoHash, CryptoHasher, Deserialize, Serialize)]
pub enum SessionId {
    Txn {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    BlockMeta {
        // block id
        id: HashValue,
    },
    Genesis {
        // id to identify this specific genesis build
        id: HashValue,
    },
    Prologue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    Epilogue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    // For those runs that are not a transaction and the output of which won't be committed.
    Void,
    RunOnAbort {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    BlockMetaExt {
        // block id
        id: HashValue,
    },
    ValidatorTxn {
        script_hash: Vec<u8>,
    },
}

impl SessionId {
    pub fn txn_meta(txn_metadata: &TransactionMetadata) -> Self {
        Self::Txn {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn genesis(id: HashValue) -> Self {
        Self::Genesis { id }
    }

    pub fn block_meta(block_meta: &BlockMetadata) -> Self {
        Self::BlockMeta {
            id: block_meta.id(),
        }
    }

    pub fn block_meta_ext(block_meta_ext: &BlockMetadataExt) -> Self {
        Self::BlockMetaExt {
            id: block_meta_ext.id(),
        }
    }

    pub fn prologue_meta(txn_metadata: &TransactionMetadata) -> Self {
        Self::Prologue {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn run_on_abort(txn_metadata: &TransactionMetadata) -> Self {
        Self::RunOnAbort {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn epilogue_meta(txn_metadata: &TransactionMetadata) -> Self {
        Self::Epilogue {
            sender: txn_metadata.sender,
            sequence_number: txn_metadata.sequence_number,
            script_hash: txn_metadata.script_hash.clone(),
        }
    }

    pub fn void() -> Self {
        Self::Void
    }

    pub fn validator_txn(txn: &ValidatorTransaction) -> Self {
        Self::ValidatorTxn {
            script_hash: txn.hash().to_vec(),
        }
    }

    pub fn as_uuid(&self) -> HashValue {
        self.hash()
    }

    pub(crate) fn into_script_hash(self) -> Vec<u8> {
        match self {
            Self::Txn {
                sender: _,
                sequence_number: _,
                script_hash,
            }
            | Self::Prologue {
                sender: _,
                sequence_number: _,
                script_hash,
            }
            | Self::Epilogue {
                sender: _,
                sequence_number: _,
                script_hash,
            }
            | Self::RunOnAbort {
                sender: _,
                sequence_number: _,
                script_hash,
            }
            | Self::ValidatorTxn { script_hash } => script_hash,
            Self::BlockMeta { id: _ }
            | Self::Genesis { id: _ }
            | Self::Void
            | Self::BlockMetaExt { id: _ } => vec![],
        }
    }
}
