// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Types and identifiers for parsing Move structs and types

use aptos_sdk::move_types::{ident_str, identifier::Identifier};

pub fn account_identifier() -> Identifier {
    ident_str!("Account").into()
}

pub fn account_identifier_lower() -> Identifier {
    ident_str!("account").into()
}

pub fn coin_identifier() -> Identifier {
    ident_str!("Coin").into()
}

pub fn coin_identifier_lower() -> Identifier {
    ident_str!("coin").into()
}
pub fn coin_info_identifier() -> Identifier {
    ident_str!("CoinInfo").into()
}

pub fn coin_store_identifier() -> Identifier {
    ident_str!("CoinStore").into()
}

pub fn create_account_identifier() -> Identifier {
    ident_str!("create_account").into()
}

pub fn aptos_coin_identifier() -> Identifier {
    ident_str!("AptosCoin").into()
}

pub fn aptos_coin_identifier_lower() -> Identifier {
    ident_str!("aptoscoin").into()
}

pub fn sequence_number_identifier() -> Identifier {
    ident_str!("sequence_number").into()
}

pub fn deposit_events_identifier() -> Identifier {
    ident_str!("deposit_events").into()
}

pub fn withdraw_events_identifier() -> Identifier {
    ident_str!("withdraw_events").into()
}

pub fn transfer_identifier() -> Identifier {
    ident_str!("transfer").into()
}

pub fn decimals_identifier() -> Identifier {
    ident_str!("decimals").into()
}

pub fn symbol_identifier() -> Identifier {
    ident_str!("symbol").into()
}
