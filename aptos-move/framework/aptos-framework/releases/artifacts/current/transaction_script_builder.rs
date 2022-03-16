// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This file was generated. Do not modify!
//
// To update this code, run: `cargo run --release -p framework`.

//! Conversion library between a structured representation of a Move script call (`ScriptCall`) and the
//! standard BCS-compatible representation used in Aptos transactions (`Script`).
//!
//! This code was generated by compiling known Script interfaces ("ABIs") with the tool `transaction-builder-generator`.

#![allow(clippy::unnecessary_wraps)]
#![allow(unused_imports)]
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Script, ScriptFunction, TransactionArgument, TransactionPayload, VecBytes},
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
};
use std::collections::BTreeMap as Map;

type Bytes = Vec<u8>;

/// Structured representation of a call into a known Move script function.
/// ```ignore
/// impl ScriptFunctionCall {
///     pub fn encode(self) -> TransactionPayload { .. }
///     pub fn decode(&TransactionPayload) -> Option<ScriptFunctionCall> { .. }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, PartialOrd)]
#[cfg_attr(feature = "fuzzing", derive(proptest_derive::Arbitrary))]
#[cfg_attr(feature = "fuzzing", proptest(no_params))]
pub enum ScriptFunctionCall {
    /// Claim the delegated mint capability and destroy the delegated token.
    ClaimMintCapability {},

    /// Basic account creation method.
    CreateAccount {
        new_account_address: AccountAddress,
        auth_key_preimage: Bytes,
    },

    CreateFiniteCollectionScript {
        token_type: TypeTag,
        description: Bytes,
        name: Bytes,
        uri: Bytes,
        maximum: u64,
    },

    CreateFiniteSimpleCollection {
        description: Bytes,
        name: Bytes,
        uri: Bytes,
        maximum: u64,
    },

    CreateSimpleToken {
        collection_name: Bytes,
        description: Bytes,
        name: Bytes,
        supply: u64,
        uri: Bytes,
    },

    CreateUnlimitedCollectionScript {
        token_type: TypeTag,
        description: Bytes,
        name: Bytes,
        uri: Bytes,
    },

    CreateUnlimitedSimpleCollection {
        description: Bytes,
        name: Bytes,
        uri: Bytes,
    },

    /// Create delegated token for the address so the account could claim MintCapability later.
    DelegateMintCapability { to: AccountAddress },

    /// Mint coins with capability.
    Mint {
        mint_addr: AccountAddress,
        amount: u64,
    },

    ReceiveFromScript {
        token_type: TypeTag,
        sender: AccountAddress,
        creator: AccountAddress,
        token_creation_num: u64,
    },

    ReceiveSimpleTokenFrom {
        sender: AccountAddress,
        creator: AccountAddress,
        token_creation_num: u64,
    },

    /// Rotate the authentication key for the account under cap.account_address
    RotateAuthenticationKey { new_authentication_key: Bytes },

    SetGasConstants {
        global_memory_per_byte_cost: u64,
        global_memory_per_byte_write_cost: u64,
        min_transaction_gas_units: u64,
        large_transaction_cutoff: u64,
        intrinsic_gas_per_byte: u64,
        maximum_number_of_gas_units: u64,
        min_price_per_gas_unit: u64,
        max_price_per_gas_unit: u64,
        max_transaction_size_in_bytes: u64,
        gas_unit_scaling_factor: u64,
        default_account_size: u64,
    },

    /// Updates the major version to a larger version.
    SetVersion { major: u64 },

    StopSimpleTokenTransferTo {
        receiver: AccountAddress,
        creator: AccountAddress,
        token_creation_num: u64,
    },

    StopTransferToScript {
        token_type: TypeTag,
        receiver: AccountAddress,
        creator: AccountAddress,
        token_creation_num: u64,
    },

    /// Transfers `amount` of tokens from `from` to `to`.
    Transfer { to: AccountAddress, amount: u64 },

    TransferSimpleTokenTo {
        receiver: AccountAddress,
        creator: AccountAddress,
        token_creation_num: u64,
        amount: u64,
    },

    TransferToScript {
        token_type: TypeTag,
        receiver: AccountAddress,
        creator: AccountAddress,
        token_creation_num: u64,
        amount: u64,
    },
}

impl ScriptFunctionCall {
    /// Build an Aptos `TransactionPayload` from a structured object `ScriptFunctionCall`.
    pub fn encode(self) -> TransactionPayload {
        use ScriptFunctionCall::*;
        match self {
            ClaimMintCapability {} => encode_claim_mint_capability_script_function(),
            CreateAccount {
                new_account_address,
                auth_key_preimage,
            } => encode_create_account_script_function(new_account_address, auth_key_preimage),
            CreateFiniteCollectionScript {
                token_type,
                description,
                name,
                uri,
                maximum,
            } => encode_create_finite_collection_script_script_function(
                token_type,
                description,
                name,
                uri,
                maximum,
            ),
            CreateFiniteSimpleCollection {
                description,
                name,
                uri,
                maximum,
            } => encode_create_finite_simple_collection_script_function(
                description,
                name,
                uri,
                maximum,
            ),
            CreateSimpleToken {
                collection_name,
                description,
                name,
                supply,
                uri,
            } => encode_create_simple_token_script_function(
                collection_name,
                description,
                name,
                supply,
                uri,
            ),
            CreateUnlimitedCollectionScript {
                token_type,
                description,
                name,
                uri,
            } => encode_create_unlimited_collection_script_script_function(
                token_type,
                description,
                name,
                uri,
            ),
            CreateUnlimitedSimpleCollection {
                description,
                name,
                uri,
            } => encode_create_unlimited_simple_collection_script_function(description, name, uri),
            DelegateMintCapability { to } => encode_delegate_mint_capability_script_function(to),
            Mint { mint_addr, amount } => encode_mint_script_function(mint_addr, amount),
            ReceiveFromScript {
                token_type,
                sender,
                creator,
                token_creation_num,
            } => encode_receive_from_script_script_function(
                token_type,
                sender,
                creator,
                token_creation_num,
            ),
            ReceiveSimpleTokenFrom {
                sender,
                creator,
                token_creation_num,
            } => encode_receive_simple_token_from_script_function(
                sender,
                creator,
                token_creation_num,
            ),
            RotateAuthenticationKey {
                new_authentication_key,
            } => encode_rotate_authentication_key_script_function(new_authentication_key),
            SetGasConstants {
                global_memory_per_byte_cost,
                global_memory_per_byte_write_cost,
                min_transaction_gas_units,
                large_transaction_cutoff,
                intrinsic_gas_per_byte,
                maximum_number_of_gas_units,
                min_price_per_gas_unit,
                max_price_per_gas_unit,
                max_transaction_size_in_bytes,
                gas_unit_scaling_factor,
                default_account_size,
            } => encode_set_gas_constants_script_function(
                global_memory_per_byte_cost,
                global_memory_per_byte_write_cost,
                min_transaction_gas_units,
                large_transaction_cutoff,
                intrinsic_gas_per_byte,
                maximum_number_of_gas_units,
                min_price_per_gas_unit,
                max_price_per_gas_unit,
                max_transaction_size_in_bytes,
                gas_unit_scaling_factor,
                default_account_size,
            ),
            SetVersion { major } => encode_set_version_script_function(major),
            StopSimpleTokenTransferTo {
                receiver,
                creator,
                token_creation_num,
            } => encode_stop_simple_token_transfer_to_script_function(
                receiver,
                creator,
                token_creation_num,
            ),
            StopTransferToScript {
                token_type,
                receiver,
                creator,
                token_creation_num,
            } => encode_stop_transfer_to_script_script_function(
                token_type,
                receiver,
                creator,
                token_creation_num,
            ),
            Transfer { to, amount } => encode_transfer_script_function(to, amount),
            TransferSimpleTokenTo {
                receiver,
                creator,
                token_creation_num,
                amount,
            } => encode_transfer_simple_token_to_script_function(
                receiver,
                creator,
                token_creation_num,
                amount,
            ),
            TransferToScript {
                token_type,
                receiver,
                creator,
                token_creation_num,
                amount,
            } => encode_transfer_to_script_script_function(
                token_type,
                receiver,
                creator,
                token_creation_num,
                amount,
            ),
        }
    }

    /// Try to recognize an Aptos `TransactionPayload` and convert it into a structured object `ScriptFunctionCall`.
    pub fn decode(payload: &TransactionPayload) -> Option<ScriptFunctionCall> {
        if let TransactionPayload::ScriptFunction(script) = payload {
            match SCRIPT_FUNCTION_DECODER_MAP.get(&format!(
                "{}{}",
                script.module().name(),
                script.function()
            )) {
                Some(decoder) => decoder(payload),
                None => None,
            }
        } else {
            None
        }
    }
}

/// Claim the delegated mint capability and destroy the delegated token.
pub fn encode_claim_mint_capability_script_function() -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TestCoin").to_owned(),
        ),
        ident_str!("claim_mint_capability").to_owned(),
        vec![],
        vec![],
    ))
}

/// Basic account creation method.
pub fn encode_create_account_script_function(
    new_account_address: AccountAddress,
    auth_key_preimage: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("AptosAccount").to_owned(),
        ),
        ident_str!("create_account").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&new_account_address).unwrap(),
            bcs::to_bytes(&auth_key_preimage).unwrap(),
        ],
    ))
}

pub fn encode_create_finite_collection_script_script_function(
    token_type: TypeTag,
    description: Vec<u8>,
    name: Vec<u8>,
    uri: Vec<u8>,
    maximum: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("Token").to_owned(),
        ),
        ident_str!("create_finite_collection_script").to_owned(),
        vec![token_type],
        vec![
            bcs::to_bytes(&description).unwrap(),
            bcs::to_bytes(&name).unwrap(),
            bcs::to_bytes(&uri).unwrap(),
            bcs::to_bytes(&maximum).unwrap(),
        ],
    ))
}

pub fn encode_create_finite_simple_collection_script_function(
    description: Vec<u8>,
    name: Vec<u8>,
    uri: Vec<u8>,
    maximum: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("SimpleToken").to_owned(),
        ),
        ident_str!("create_finite_simple_collection").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&description).unwrap(),
            bcs::to_bytes(&name).unwrap(),
            bcs::to_bytes(&uri).unwrap(),
            bcs::to_bytes(&maximum).unwrap(),
        ],
    ))
}

pub fn encode_create_simple_token_script_function(
    collection_name: Vec<u8>,
    description: Vec<u8>,
    name: Vec<u8>,
    supply: u64,
    uri: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("SimpleToken").to_owned(),
        ),
        ident_str!("create_simple_token").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&collection_name).unwrap(),
            bcs::to_bytes(&description).unwrap(),
            bcs::to_bytes(&name).unwrap(),
            bcs::to_bytes(&supply).unwrap(),
            bcs::to_bytes(&uri).unwrap(),
        ],
    ))
}

pub fn encode_create_unlimited_collection_script_script_function(
    token_type: TypeTag,
    description: Vec<u8>,
    name: Vec<u8>,
    uri: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("Token").to_owned(),
        ),
        ident_str!("create_unlimited_collection_script").to_owned(),
        vec![token_type],
        vec![
            bcs::to_bytes(&description).unwrap(),
            bcs::to_bytes(&name).unwrap(),
            bcs::to_bytes(&uri).unwrap(),
        ],
    ))
}

pub fn encode_create_unlimited_simple_collection_script_function(
    description: Vec<u8>,
    name: Vec<u8>,
    uri: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("SimpleToken").to_owned(),
        ),
        ident_str!("create_unlimited_simple_collection").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&description).unwrap(),
            bcs::to_bytes(&name).unwrap(),
            bcs::to_bytes(&uri).unwrap(),
        ],
    ))
}

/// Create delegated token for the address so the account could claim MintCapability later.
pub fn encode_delegate_mint_capability_script_function(to: AccountAddress) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TestCoin").to_owned(),
        ),
        ident_str!("delegate_mint_capability").to_owned(),
        vec![],
        vec![bcs::to_bytes(&to).unwrap()],
    ))
}

/// Mint coins with capability.
pub fn encode_mint_script_function(mint_addr: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TestCoin").to_owned(),
        ),
        ident_str!("mint").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&mint_addr).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

pub fn encode_receive_from_script_script_function(
    token_type: TypeTag,
    sender: AccountAddress,
    creator: AccountAddress,
    token_creation_num: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TokenTransfers").to_owned(),
        ),
        ident_str!("receive_from_script").to_owned(),
        vec![token_type],
        vec![
            bcs::to_bytes(&sender).unwrap(),
            bcs::to_bytes(&creator).unwrap(),
            bcs::to_bytes(&token_creation_num).unwrap(),
        ],
    ))
}

pub fn encode_receive_simple_token_from_script_function(
    sender: AccountAddress,
    creator: AccountAddress,
    token_creation_num: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("SimpleToken").to_owned(),
        ),
        ident_str!("receive_simple_token_from").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&sender).unwrap(),
            bcs::to_bytes(&creator).unwrap(),
            bcs::to_bytes(&token_creation_num).unwrap(),
        ],
    ))
}

/// Rotate the authentication key for the account under cap.account_address
pub fn encode_rotate_authentication_key_script_function(
    new_authentication_key: Vec<u8>,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("AptosAccount").to_owned(),
        ),
        ident_str!("rotate_authentication_key").to_owned(),
        vec![],
        vec![bcs::to_bytes(&new_authentication_key).unwrap()],
    ))
}

pub fn encode_set_gas_constants_script_function(
    global_memory_per_byte_cost: u64,
    global_memory_per_byte_write_cost: u64,
    min_transaction_gas_units: u64,
    large_transaction_cutoff: u64,
    intrinsic_gas_per_byte: u64,
    maximum_number_of_gas_units: u64,
    min_price_per_gas_unit: u64,
    max_price_per_gas_unit: u64,
    max_transaction_size_in_bytes: u64,
    gas_unit_scaling_factor: u64,
    default_account_size: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("AptosVMConfig").to_owned(),
        ),
        ident_str!("set_gas_constants").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&global_memory_per_byte_cost).unwrap(),
            bcs::to_bytes(&global_memory_per_byte_write_cost).unwrap(),
            bcs::to_bytes(&min_transaction_gas_units).unwrap(),
            bcs::to_bytes(&large_transaction_cutoff).unwrap(),
            bcs::to_bytes(&intrinsic_gas_per_byte).unwrap(),
            bcs::to_bytes(&maximum_number_of_gas_units).unwrap(),
            bcs::to_bytes(&min_price_per_gas_unit).unwrap(),
            bcs::to_bytes(&max_price_per_gas_unit).unwrap(),
            bcs::to_bytes(&max_transaction_size_in_bytes).unwrap(),
            bcs::to_bytes(&gas_unit_scaling_factor).unwrap(),
            bcs::to_bytes(&default_account_size).unwrap(),
        ],
    ))
}

/// Updates the major version to a larger version.
pub fn encode_set_version_script_function(major: u64) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("AptosVersion").to_owned(),
        ),
        ident_str!("set_version").to_owned(),
        vec![],
        vec![bcs::to_bytes(&major).unwrap()],
    ))
}

pub fn encode_stop_simple_token_transfer_to_script_function(
    receiver: AccountAddress,
    creator: AccountAddress,
    token_creation_num: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("SimpleToken").to_owned(),
        ),
        ident_str!("stop_simple_token_transfer_to").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&receiver).unwrap(),
            bcs::to_bytes(&creator).unwrap(),
            bcs::to_bytes(&token_creation_num).unwrap(),
        ],
    ))
}

pub fn encode_stop_transfer_to_script_script_function(
    token_type: TypeTag,
    receiver: AccountAddress,
    creator: AccountAddress,
    token_creation_num: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TokenTransfers").to_owned(),
        ),
        ident_str!("stop_transfer_to_script").to_owned(),
        vec![token_type],
        vec![
            bcs::to_bytes(&receiver).unwrap(),
            bcs::to_bytes(&creator).unwrap(),
            bcs::to_bytes(&token_creation_num).unwrap(),
        ],
    ))
}

/// Transfers `amount` of tokens from `from` to `to`.
pub fn encode_transfer_script_function(to: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TestCoin").to_owned(),
        ),
        ident_str!("transfer").to_owned(),
        vec![],
        vec![bcs::to_bytes(&to).unwrap(), bcs::to_bytes(&amount).unwrap()],
    ))
}

pub fn encode_transfer_simple_token_to_script_function(
    receiver: AccountAddress,
    creator: AccountAddress,
    token_creation_num: u64,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("SimpleToken").to_owned(),
        ),
        ident_str!("transfer_simple_token_to").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&receiver).unwrap(),
            bcs::to_bytes(&creator).unwrap(),
            bcs::to_bytes(&token_creation_num).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

pub fn encode_transfer_to_script_script_function(
    token_type: TypeTag,
    receiver: AccountAddress,
    creator: AccountAddress,
    token_creation_num: u64,
    amount: u64,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("TokenTransfers").to_owned(),
        ),
        ident_str!("transfer_to_script").to_owned(),
        vec![token_type],
        vec![
            bcs::to_bytes(&receiver).unwrap(),
            bcs::to_bytes(&creator).unwrap(),
            bcs::to_bytes(&token_creation_num).unwrap(),
            bcs::to_bytes(&amount).unwrap(),
        ],
    ))
}

fn decode_claim_mint_capability_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(_script) = payload {
        Some(ScriptFunctionCall::ClaimMintCapability {})
    } else {
        None
    }
}

fn decode_create_account_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::CreateAccount {
            new_account_address: bcs::from_bytes(script.args().get(0)?).ok()?,
            auth_key_preimage: bcs::from_bytes(script.args().get(1)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_create_finite_collection_script_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::CreateFiniteCollectionScript {
            token_type: script.ty_args().get(0)?.clone(),
            description: bcs::from_bytes(script.args().get(0)?).ok()?,
            name: bcs::from_bytes(script.args().get(1)?).ok()?,
            uri: bcs::from_bytes(script.args().get(2)?).ok()?,
            maximum: bcs::from_bytes(script.args().get(3)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_create_finite_simple_collection_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::CreateFiniteSimpleCollection {
            description: bcs::from_bytes(script.args().get(0)?).ok()?,
            name: bcs::from_bytes(script.args().get(1)?).ok()?,
            uri: bcs::from_bytes(script.args().get(2)?).ok()?,
            maximum: bcs::from_bytes(script.args().get(3)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_create_simple_token_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::CreateSimpleToken {
            collection_name: bcs::from_bytes(script.args().get(0)?).ok()?,
            description: bcs::from_bytes(script.args().get(1)?).ok()?,
            name: bcs::from_bytes(script.args().get(2)?).ok()?,
            supply: bcs::from_bytes(script.args().get(3)?).ok()?,
            uri: bcs::from_bytes(script.args().get(4)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_create_unlimited_collection_script_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::CreateUnlimitedCollectionScript {
            token_type: script.ty_args().get(0)?.clone(),
            description: bcs::from_bytes(script.args().get(0)?).ok()?,
            name: bcs::from_bytes(script.args().get(1)?).ok()?,
            uri: bcs::from_bytes(script.args().get(2)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_create_unlimited_simple_collection_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::CreateUnlimitedSimpleCollection {
            description: bcs::from_bytes(script.args().get(0)?).ok()?,
            name: bcs::from_bytes(script.args().get(1)?).ok()?,
            uri: bcs::from_bytes(script.args().get(2)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_delegate_mint_capability_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::DelegateMintCapability {
            to: bcs::from_bytes(script.args().get(0)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_mint_script_function(payload: &TransactionPayload) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::Mint {
            mint_addr: bcs::from_bytes(script.args().get(0)?).ok()?,
            amount: bcs::from_bytes(script.args().get(1)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_receive_from_script_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::ReceiveFromScript {
            token_type: script.ty_args().get(0)?.clone(),
            sender: bcs::from_bytes(script.args().get(0)?).ok()?,
            creator: bcs::from_bytes(script.args().get(1)?).ok()?,
            token_creation_num: bcs::from_bytes(script.args().get(2)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_receive_simple_token_from_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::ReceiveSimpleTokenFrom {
            sender: bcs::from_bytes(script.args().get(0)?).ok()?,
            creator: bcs::from_bytes(script.args().get(1)?).ok()?,
            token_creation_num: bcs::from_bytes(script.args().get(2)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_rotate_authentication_key_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::RotateAuthenticationKey {
            new_authentication_key: bcs::from_bytes(script.args().get(0)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_set_gas_constants_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::SetGasConstants {
            global_memory_per_byte_cost: bcs::from_bytes(script.args().get(0)?).ok()?,
            global_memory_per_byte_write_cost: bcs::from_bytes(script.args().get(1)?).ok()?,
            min_transaction_gas_units: bcs::from_bytes(script.args().get(2)?).ok()?,
            large_transaction_cutoff: bcs::from_bytes(script.args().get(3)?).ok()?,
            intrinsic_gas_per_byte: bcs::from_bytes(script.args().get(4)?).ok()?,
            maximum_number_of_gas_units: bcs::from_bytes(script.args().get(5)?).ok()?,
            min_price_per_gas_unit: bcs::from_bytes(script.args().get(6)?).ok()?,
            max_price_per_gas_unit: bcs::from_bytes(script.args().get(7)?).ok()?,
            max_transaction_size_in_bytes: bcs::from_bytes(script.args().get(8)?).ok()?,
            gas_unit_scaling_factor: bcs::from_bytes(script.args().get(9)?).ok()?,
            default_account_size: bcs::from_bytes(script.args().get(10)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_set_version_script_function(payload: &TransactionPayload) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::SetVersion {
            major: bcs::from_bytes(script.args().get(0)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_stop_simple_token_transfer_to_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::StopSimpleTokenTransferTo {
            receiver: bcs::from_bytes(script.args().get(0)?).ok()?,
            creator: bcs::from_bytes(script.args().get(1)?).ok()?,
            token_creation_num: bcs::from_bytes(script.args().get(2)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_stop_transfer_to_script_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::StopTransferToScript {
            token_type: script.ty_args().get(0)?.clone(),
            receiver: bcs::from_bytes(script.args().get(0)?).ok()?,
            creator: bcs::from_bytes(script.args().get(1)?).ok()?,
            token_creation_num: bcs::from_bytes(script.args().get(2)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_transfer_script_function(payload: &TransactionPayload) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::Transfer {
            to: bcs::from_bytes(script.args().get(0)?).ok()?,
            amount: bcs::from_bytes(script.args().get(1)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_transfer_simple_token_to_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::TransferSimpleTokenTo {
            receiver: bcs::from_bytes(script.args().get(0)?).ok()?,
            creator: bcs::from_bytes(script.args().get(1)?).ok()?,
            token_creation_num: bcs::from_bytes(script.args().get(2)?).ok()?,
            amount: bcs::from_bytes(script.args().get(3)?).ok()?,
        })
    } else {
        None
    }
}

fn decode_transfer_to_script_script_function(
    payload: &TransactionPayload,
) -> Option<ScriptFunctionCall> {
    if let TransactionPayload::ScriptFunction(script) = payload {
        Some(ScriptFunctionCall::TransferToScript {
            token_type: script.ty_args().get(0)?.clone(),
            receiver: bcs::from_bytes(script.args().get(0)?).ok()?,
            creator: bcs::from_bytes(script.args().get(1)?).ok()?,
            token_creation_num: bcs::from_bytes(script.args().get(2)?).ok()?,
            amount: bcs::from_bytes(script.args().get(3)?).ok()?,
        })
    } else {
        None
    }
}

type ScriptFunctionDecoderMap = std::collections::HashMap<
    String,
    Box<
        dyn Fn(&TransactionPayload) -> Option<ScriptFunctionCall>
            + std::marker::Sync
            + std::marker::Send,
    >,
>;

static SCRIPT_FUNCTION_DECODER_MAP: once_cell::sync::Lazy<ScriptFunctionDecoderMap> =
    once_cell::sync::Lazy::new(|| {
        let mut map: ScriptFunctionDecoderMap = std::collections::HashMap::new();
        map.insert(
            "TestCoinclaim_mint_capability".to_string(),
            Box::new(decode_claim_mint_capability_script_function),
        );
        map.insert(
            "AptosAccountcreate_account".to_string(),
            Box::new(decode_create_account_script_function),
        );
        map.insert(
            "Tokencreate_finite_collection_script".to_string(),
            Box::new(decode_create_finite_collection_script_script_function),
        );
        map.insert(
            "SimpleTokencreate_finite_simple_collection".to_string(),
            Box::new(decode_create_finite_simple_collection_script_function),
        );
        map.insert(
            "SimpleTokencreate_simple_token".to_string(),
            Box::new(decode_create_simple_token_script_function),
        );
        map.insert(
            "Tokencreate_unlimited_collection_script".to_string(),
            Box::new(decode_create_unlimited_collection_script_script_function),
        );
        map.insert(
            "SimpleTokencreate_unlimited_simple_collection".to_string(),
            Box::new(decode_create_unlimited_simple_collection_script_function),
        );
        map.insert(
            "TestCoindelegate_mint_capability".to_string(),
            Box::new(decode_delegate_mint_capability_script_function),
        );
        map.insert(
            "TestCoinmint".to_string(),
            Box::new(decode_mint_script_function),
        );
        map.insert(
            "TokenTransfersreceive_from_script".to_string(),
            Box::new(decode_receive_from_script_script_function),
        );
        map.insert(
            "SimpleTokenreceive_simple_token_from".to_string(),
            Box::new(decode_receive_simple_token_from_script_function),
        );
        map.insert(
            "AptosAccountrotate_authentication_key".to_string(),
            Box::new(decode_rotate_authentication_key_script_function),
        );
        map.insert(
            "AptosVMConfigset_gas_constants".to_string(),
            Box::new(decode_set_gas_constants_script_function),
        );
        map.insert(
            "AptosVersionset_version".to_string(),
            Box::new(decode_set_version_script_function),
        );
        map.insert(
            "SimpleTokenstop_simple_token_transfer_to".to_string(),
            Box::new(decode_stop_simple_token_transfer_to_script_function),
        );
        map.insert(
            "TokenTransfersstop_transfer_to_script".to_string(),
            Box::new(decode_stop_transfer_to_script_script_function),
        );
        map.insert(
            "TestCointransfer".to_string(),
            Box::new(decode_transfer_script_function),
        );
        map.insert(
            "SimpleTokentransfer_simple_token_to".to_string(),
            Box::new(decode_transfer_simple_token_to_script_function),
        );
        map.insert(
            "TokenTransferstransfer_to_script".to_string(),
            Box::new(decode_transfer_to_script_script_function),
        );
        map
    });
