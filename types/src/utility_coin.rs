// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use move_deps::move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use once_cell::sync::Lazy;

pub static TEST_COIN_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("test_coin").to_owned(),
        name: ident_str!("TestCoin").to_owned(),
        type_params: vec![],
    })
});
