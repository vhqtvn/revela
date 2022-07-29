// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This file is where we apply a number of traits that allow us to use these
//! traits with Poem. For more information on how these macros work, see the
//! documentation within `crates/aptos-openapi`.
//!
//! For potential future improvements here, see:
//! https://github.com/aptos-labs/aptos-core/issues/2319.

use aptos_openapi::{impl_poem_parameter, impl_poem_type};
use serde_json::json;

use crate::{
    move_types::{MoveAbility, MoveStructValue},
    Address, EventKey, HashValue, HexEncodedBytes, IdentifierWrapper, MoveStructTagWrapper,
    MoveType, U128, U64,
};
use indoc::indoc;

impl_poem_type!(
    Address,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1".to_string()
        )),
        format = Some("hex"),
        description = Some("Hex encoded 32 byte Aptos account address")
    )
);

impl_poem_type!(
    EventKey,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x000000000000000088fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1"
                .to_string()
        )),
        format = Some("hex"),
        description = Some(indoc! {"
            Event key is a global index for an event stream.

            It is hex-encoded BCS bytes of `EventHandle` `guid` field value, which is
            a combination of a `uint64` creation number and account address (without
            trimming leading zeros).

            For example, event key `0x000000000000000088fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1` is combined by the following 2 parts:
              1. `0000000000000000`: `uint64` representation of `0`.
              2. `88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1`: 32 bytes of account address.
        "})
    )
);

impl_poem_type!(HashValue, "string", ());

impl_poem_type!(
    HexEncodedBytes,
    "string",
    (
        example = Some(serde_json::Value::String(
            "0x88fbd33f54e1126269769780feb24480428179f552e2313fbe571b72e62a1ca1".to_string()
        )),
        format = Some("hex"),
        description = Some(indoc! {"
            All bytes (Vec<u8>) data is represented as hex-encoded string prefixed with `0x` and fulfilled with
            two hex digits per byte.

            Unlike the `Address` type, HexEncodedBytes will not trim any zeros.
        "})
    )
);

impl_poem_type!(IdentifierWrapper, "string", ());

impl_poem_type!(MoveAbility, "string", ());

impl_poem_type!(
    MoveStructValue,
    "object",
    (
        example = Some(json!({
            "authentication_key": "0x0000000000000000000000000000000000000000000000000000000000000001",
            "coin_register_events": {
              "counter": "0",
              "guid": {
                "id": {
                  "addr": "0x1",
                  "creation_num": "0"
                }
              }
            },
            "self_address": "0x1",
            "sequence_number": "0"
        })),
        description = Some(indoc! {"
            This is a JSON representation of some data within an account resource. More specifically,
            it is a map of strings to arbitrary JSON values / objects, where the keys are top level
            fields within the given resource.

            To clarify, you might query for 0x1::account::Account and see the example data.

            Move `bool` type value is serialized into `boolean`.

            Move `u8` type value is serialized into `integer`.

            Move `u64` and `u128` type value is serialized into `string`.

            Move `address` type value (32 byte Aptos account address) is serialized into a HexEncodedBytes string.
            For example:
              - `0x1`
              - `0x1668f6be25668c1a17cd8caf6b8d2f25`

            Move `vector` type value is serialized into `array`, except `vector<u8>` which is serialized into a
            HexEncodedBytes string with `0x` prefix.
            For example:
              - `vector<u64>{255, 255}` => `[\"255\", \"255\"]`
              - `vector<u8>{255, 255}` => `0xffff`

            Move `struct` type value is serialized into `object` that looks like this (except some Move stdlib types, see the following section):
              ```json
              {
                field1_name: field1_value,
                field2_name: field2_value,
                ......
              }
              ```

            For example:
              `{ \"created\": \"0xa550c18\", \"role_id\": \"0\" }`

            **Special serialization for Move stdlib types**:
              - [0x1::string::String](https://github.com/aptos-labs/aptos-core/blob/main/language/move-stdlib/docs/ascii.md)
                is serialized into `string`. For example, struct value `0x1::string::String{bytes: b\"Hello World!\"}`
                is serialized as `\"Hello World!\"` in JSON.
        "})
    )
);

impl_poem_type!(
    MoveType,
    "string",
    (
        pattern =
            Some("^(bool|u8|u64|u128|address|signer|vector<.+>|0x[0-9a-zA-Z:_<, >]+)$".to_string()),
        description = Some(indoc! {"
            String representation of an on-chain Move type tag that is exposed in transaction payload.
                Values:
                  - bool
                  - u8
                  - u64
                  - u128
                  - address
                  - signer
                  - vector: `vector<{non-reference MoveTypeId}>`
                  - struct: `{address}::{module_name}::{struct_name}::<{generic types}>`

                Vector type value examples:
                  - `vector<u8>`
                  - `vector<vector<u64>>`
                  - `vector<0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>>`

                Struct type value examples:
                  - `0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>
                  - `0x1::account::Account`

                Note:
                  1. Empty chars should be ignored when comparing 2 struct tag ids.
                  2. When used in an URL path, should be encoded by url-encoding (AKA percent-encoding).
    "})
    )
);

impl_poem_type!(
    U64,
    "string",
    (
        example = Some(serde_json::Value::String("32425224034".to_string())),
        format = Some("uint64"),
        description = Some(indoc! {"
        A string containing a 64-bit unsigned integer.

        We represent u64 values as a string to ensure compatability with languages such
        as JavaScript that do not parse u64s in JSON natively.
    "})
    )
);

impl_poem_type!(
    U128,
    "string",
    (
        example = Some(serde_json::Value::String(
            "340282366920938463463374607431768211454".to_string()
        )),
        format = Some("uint64"),
        description = Some(indoc! {"
        A string containing a 128-bit unsigned integer.

        We represent u128 values as a string to ensure compatability with languages such
        as JavaScript that do not parse u64s in JSON natively.
    "})
    )
);

impl_poem_parameter!(
    Address,
    EventKey,
    HashValue,
    IdentifierWrapper,
    HexEncodedBytes,
    MoveStructTagWrapper,
    U64,
    U128
);
