// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! This test verifies the behavior of the Canonical Serializer against the test vectors
//! provided in the Canonical Serializer document.
//! See: common/canonical_serializer/README.md
//!
//! Test data was generated as follows:
//! - AccountAddress: AccountAddress::random()
//! - u64: rand::thread_rng().gen::<u64>()
//!
//! To retrieve the binary output, the types were then passed to the following function:
//!
//! fn serialize<T>(input: T)
//!     where T: CanonicalSerialize + std::fmt::Debug
//! {
//!     let mut serializer = SimpleSerializer::<Vec<u8>>::new();
//!     serializer.encode_struct(&input).unwrap();
//!     let output = serializer.get_output();
//!     println!("{:?} = {:?}", input, hex::encode_upper(output));
//! }
//!
//! Both input represented as hex arrays, specifically AccountAddress and path, and the
//! serialized output were passed to the following formatting script to make Rust arrays:
//!
//! #!/usr/bin/env python3
//!
//! import fileinput
//!
//! indata = fileinput.input().readline().strip()
//! outdata = ""
//!
//! for idx in range(0, len(indata), 2):
//!     outdata += "0x{}{}, ".format(indata[idx], indata[idx+1])
//!
//! print(outdata[:-2])

use crate::{
    access_path::AccessPath,
    account_address::AccountAddress,
    byte_array::ByteArray,
    transaction::{RawTransaction, Script, TransactionArgument, TransactionPayload},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use canonical_serialization::SimpleSerializer;
use std::time::Duration;

#[test]
fn test_access_path_canonical_serialization_example() {
    let account_address = AccountAddress::new([
        0x9a, 0x1a, 0xd0, 0x97, 0x42, 0xd1, 0xff, 0xc6, 0x2e, 0x65, 0x9e, 0x9a, 0x77, 0x97, 0x80,
        0x8b, 0x20, 0x6f, 0x95, 0x6f, 0x13, 0x1d, 0x07, 0x50, 0x94, 0x49, 0xc0, 0x1a, 0xd8, 0x22,
        0x0a, 0xd4,
    ]);
    let input = AccessPath::new(
        account_address,
        vec![
            0x01, 0x21, 0x7d, 0xa6, 0xc6, 0xb3, 0xe1, 0x9f, 0x18, 0x25, 0xcf, 0xb2, 0x67, 0x6d,
            0xae, 0xcc, 0xe3, 0xbf, 0x3d, 0xe0, 0x3c, 0xf2, 0x66, 0x47, 0xc7, 0x8d, 0xf0, 0x0b,
            0x37, 0x1b, 0x25, 0xcc, 0x97,
        ],
    );

    let expected_output = vec![
        0x20, 0x00, 0x00, 0x00, 0x9A, 0x1A, 0xD0, 0x97, 0x42, 0xD1, 0xFF, 0xC6, 0x2E, 0x65, 0x9E,
        0x9A, 0x77, 0x97, 0x80, 0x8B, 0x20, 0x6F, 0x95, 0x6F, 0x13, 0x1D, 0x07, 0x50, 0x94, 0x49,
        0xC0, 0x1A, 0xD8, 0x22, 0x0A, 0xD4, 0x21, 0x00, 0x00, 0x00, 0x01, 0x21, 0x7D, 0xA6, 0xC6,
        0xB3, 0xE1, 0x9F, 0x18, 0x25, 0xCF, 0xB2, 0x67, 0x6D, 0xAE, 0xCC, 0xE3, 0xBF, 0x3D, 0xE0,
        0x3C, 0xF2, 0x66, 0x47, 0xC7, 0x8D, 0xF0, 0x0B, 0x37, 0x1B, 0x25, 0xCC, 0x97,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_account_address_canonical_serialization_example() {
    let input = AccountAddress::new([
        0xca, 0x82, 0x0b, 0xf9, 0x30, 0x5e, 0xb9, 0x7d, 0x0d, 0x78, 0x4f, 0x71, 0xb3, 0x95, 0x54,
        0x57, 0xfb, 0xf6, 0x91, 0x1f, 0x53, 0x00, 0xce, 0xaa, 0x5d, 0x7e, 0x86, 0x21, 0x52, 0x9e,
        0xae, 0x19,
    ]);

    let expected_output: Vec<u8> = vec![
        0x20, 0x00, 0x00, 0x00, 0xCA, 0x82, 0x0B, 0xF9, 0x30, 0x5E, 0xB9, 0x7D, 0x0D, 0x78, 0x4F,
        0x71, 0xB3, 0x95, 0x54, 0x57, 0xFB, 0xF6, 0x91, 0x1F, 0x53, 0x00, 0xCE, 0xAA, 0x5D, 0x7E,
        0x86, 0x21, 0x52, 0x9E, 0xAE, 0x19,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_program_canonical_serialization_example() {
    let input = get_common_program();

    let expected_output: Vec<u8> = vec![
        0x04, 0x00, 0x00, 0x00, 0x6D, 0x6F, 0x76, 0x65, 0x02, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00,
        0x00, 0x09, 0x00, 0x00, 0x00, 0x43, 0x41, 0x46, 0x45, 0x20, 0x44, 0x30, 0x30, 0x44, 0x02,
        0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x63, 0x61, 0x66, 0x65, 0x20, 0x64, 0x30, 0x30,
        0x64,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_raw_transaction_with_a_program_canonical_serialization_example() {
    let input = RawTransaction::new_script(
        AccountAddress::new([
            0x3a, 0x24, 0xa6, 0x1e, 0x05, 0xd1, 0x29, 0xca, 0xce, 0x9e, 0x0e, 0xfc, 0x8b, 0xc9,
            0xe3, 0x38, 0x31, 0xfe, 0xc9, 0xa9, 0xbe, 0x66, 0xf5, 0x0f, 0xd3, 0x52, 0xa2, 0x63,
            0x8a, 0x49, 0xb9, 0xee,
        ]),
        32,
        get_common_program(),
        10000,
        20000,
        Duration::from_secs(86400),
    );

    let expected_output = vec![
        0x20, 0x00, 0x00, 0x00, 0x3A, 0x24, 0xA6, 0x1E, 0x05, 0xD1, 0x29, 0xCA, 0xCE, 0x9E, 0x0E,
        0xFC, 0x8B, 0xC9, 0xE3, 0x38, 0x31, 0xFE, 0xC9, 0xA9, 0xBE, 0x66, 0xF5, 0x0F, 0xD3, 0x52,
        0xA2, 0x63, 0x8A, 0x49, 0xB9, 0xEE, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x6D, 0x6F, 0x76, 0x65, 0x02, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x43, 0x41, 0x46, 0x45, 0x20, 0x44, 0x30,
        0x30, 0x44, 0x02, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x63, 0x61, 0x66, 0x65, 0x20,
        0x64, 0x30, 0x30, 0x64, 0x10, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x20, 0x4E, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x51, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_raw_transaction_with_a_write_set_canonical_serialization_example() {
    let input = RawTransaction::new_write_set(
        AccountAddress::new([
            0xc3, 0x39, 0x8a, 0x59, 0x9a, 0x6f, 0x3b, 0x9f, 0x30, 0xb6, 0x35, 0xaf, 0x29, 0xf2,
            0xba, 0x04, 0x6d, 0x3a, 0x75, 0x2c, 0x26, 0xe9, 0xd0, 0x64, 0x7b, 0x96, 0x47, 0xd1,
            0xf4, 0xc0, 0x4a, 0xd4,
        ]),
        32,
        get_common_write_set(),
    );

    let expected_output = vec![
        0x20, 0x00, 0x00, 0x00, 0xC3, 0x39, 0x8A, 0x59, 0x9A, 0x6F, 0x3B, 0x9F, 0x30, 0xB6, 0x35,
        0xAF, 0x29, 0xF2, 0xBA, 0x04, 0x6D, 0x3A, 0x75, 0x2C, 0x26, 0xE9, 0xD0, 0x64, 0x7B, 0x96,
        0x47, 0xD1, 0xF4, 0xC0, 0x4A, 0xD4, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xA7, 0x1D, 0x76, 0xFA,
        0xA2, 0xD2, 0xD5, 0xC3, 0x22, 0x4E, 0xC3, 0xD4, 0x1D, 0xEB, 0x29, 0x39, 0x73, 0x56, 0x4A,
        0x79, 0x1E, 0x55, 0xC6, 0x78, 0x2B, 0xA7, 0x6C, 0x2B, 0xF0, 0x49, 0x5F, 0x9A, 0x21, 0x00,
        0x00, 0x00, 0x01, 0x21, 0x7D, 0xA6, 0xC6, 0xB3, 0xE1, 0x9F, 0x18, 0x25, 0xCF, 0xB2, 0x67,
        0x6D, 0xAE, 0xCC, 0xE3, 0xBF, 0x3D, 0xE0, 0x3C, 0xF2, 0x66, 0x47, 0xC7, 0x8D, 0xF0, 0x0B,
        0x37, 0x1B, 0x25, 0xCC, 0x97, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xC4, 0xC6,
        0x3F, 0x80, 0xC7, 0x4B, 0x11, 0x26, 0x3E, 0x42, 0x1E, 0xBF, 0x84, 0x86, 0xA4, 0xE3, 0x98,
        0xD0, 0xDB, 0xC0, 0x9F, 0xA7, 0xD4, 0xF6, 0x2C, 0xCD, 0xB3, 0x09, 0xF3, 0xAE, 0xA8, 0x1F,
        0x09, 0x00, 0x00, 0x00, 0x01, 0x21, 0x7D, 0xA6, 0xC6, 0xB3, 0xE1, 0x9F, 0x18, 0x01, 0x00,
        0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xCA, 0xFE, 0xD0, 0x0D, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
        0xFF, 0xFF, 0xFF, 0xFF,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_transaction_argument_address_canonical_serialization_example() {
    let input = TransactionArgument::Address(AccountAddress::new([
        0x2c, 0x25, 0x99, 0x17, 0x85, 0x34, 0x3b, 0x23, 0xae, 0x07, 0x3a, 0x50, 0xe5, 0xfd, 0x80,
        0x9a, 0x2c, 0xd8, 0x67, 0x52, 0x6b, 0x3c, 0x1d, 0xb2, 0xb0, 0xbf, 0x5d, 0x19, 0x24, 0xc6,
        0x93, 0xed,
    ]));

    let expected_output: Vec<u8> = vec![
        0x01, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x2C, 0x25, 0x99, 0x17, 0x85, 0x34, 0x3B,
        0x23, 0xAE, 0x07, 0x3A, 0x50, 0xE5, 0xFD, 0x80, 0x9A, 0x2C, 0xD8, 0x67, 0x52, 0x6B, 0x3C,
        0x1D, 0xB2, 0xB0, 0xBF, 0x5D, 0x19, 0x24, 0xC6, 0x93, 0xED,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_transaction_argument_byte_array_canonical_serialization_example() {
    let input = TransactionArgument::ByteArray(ByteArray::new(vec![0xCA, 0xFE, 0xD0, 0x0D]));

    let expected_output: Vec<u8> = vec![
        0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xCA, 0xFE, 0xD0, 0x0D,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_transaction_argument_string_canonical_serialization_example() {
    let input = TransactionArgument::String("Hello, World!".to_string());
    let expected_output: Vec<u8> = vec![
        0x02, 0x00, 0x00, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x2C, 0x20,
        0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_transaction_argument_u64_canonical_serialization_example() {
    let input = TransactionArgument::U64(9_213_671_392_124_193_148);
    let expected_output: Vec<u8> = vec![
        0x00, 0x00, 0x00, 0x00, 0x7C, 0xC9, 0xBD, 0xA4, 0x50, 0x89, 0xDD, 0x7F,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_transaction_payload_with_a_program_canonical_serialization_example() {
    let input = TransactionPayload::Script(get_common_program());

    let expected_output = vec![
        0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x6D, 0x6F, 0x76, 0x65, 0x02, 0x00, 0x00,
        0x00, 0x02, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x43, 0x41, 0x46, 0x45, 0x20, 0x44,
        0x30, 0x30, 0x44, 0x02, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x63, 0x61, 0x66, 0x65,
        0x20, 0x64, 0x30, 0x30, 0x64,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_transaction_payload_with_a_write_set_canonical_serialization_example() {
    let input = TransactionPayload::WriteSet(get_common_write_set());

    let expected_output = vec![
        0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xA7, 0x1D, 0x76,
        0xFA, 0xA2, 0xD2, 0xD5, 0xC3, 0x22, 0x4E, 0xC3, 0xD4, 0x1D, 0xEB, 0x29, 0x39, 0x73, 0x56,
        0x4A, 0x79, 0x1E, 0x55, 0xC6, 0x78, 0x2B, 0xA7, 0x6C, 0x2B, 0xF0, 0x49, 0x5F, 0x9A, 0x21,
        0x00, 0x00, 0x00, 0x01, 0x21, 0x7D, 0xA6, 0xC6, 0xB3, 0xE1, 0x9F, 0x18, 0x25, 0xCF, 0xB2,
        0x67, 0x6D, 0xAE, 0xCC, 0xE3, 0xBF, 0x3D, 0xE0, 0x3C, 0xF2, 0x66, 0x47, 0xC7, 0x8D, 0xF0,
        0x0B, 0x37, 0x1B, 0x25, 0xCC, 0x97, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xC4,
        0xC6, 0x3F, 0x80, 0xC7, 0x4B, 0x11, 0x26, 0x3E, 0x42, 0x1E, 0xBF, 0x84, 0x86, 0xA4, 0xE3,
        0x98, 0xD0, 0xDB, 0xC0, 0x9F, 0xA7, 0xD4, 0xF6, 0x2C, 0xCD, 0xB3, 0x09, 0xF3, 0xAE, 0xA8,
        0x1F, 0x09, 0x00, 0x00, 0x00, 0x01, 0x21, 0x7D, 0xA6, 0xC6, 0xB3, 0xE1, 0x9F, 0x18, 0x01,
        0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xCA, 0xFE, 0xD0, 0x0D,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_write_op_delete_canonical_serialization_example() {
    let input = WriteOp::Deletion;
    let expected_output = vec![0x00, 0x00, 0x00, 0x00];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_write_op_value_canonical_serialization_example() {
    let input = WriteOp::Value(vec![0xca, 0xfe, 0xd0, 0x0d]);
    let expected_output = vec![
        0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xCA, 0xFE, 0xD0, 0x0D,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

#[test]
fn test_write_set_canonical_serialization_example() {
    let input = get_common_write_set();

    let expected_output = vec![
        0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xA7, 0x1D, 0x76, 0xFA, 0xA2, 0xD2, 0xD5,
        0xC3, 0x22, 0x4E, 0xC3, 0xD4, 0x1D, 0xEB, 0x29, 0x39, 0x73, 0x56, 0x4A, 0x79, 0x1E, 0x55,
        0xC6, 0x78, 0x2B, 0xA7, 0x6C, 0x2B, 0xF0, 0x49, 0x5F, 0x9A, 0x21, 0x00, 0x00, 0x00, 0x01,
        0x21, 0x7D, 0xA6, 0xC6, 0xB3, 0xE1, 0x9F, 0x18, 0x25, 0xCF, 0xB2, 0x67, 0x6D, 0xAE, 0xCC,
        0xE3, 0xBF, 0x3D, 0xE0, 0x3C, 0xF2, 0x66, 0x47, 0xC7, 0x8D, 0xF0, 0x0B, 0x37, 0x1B, 0x25,
        0xCC, 0x97, 0x00, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0xC4, 0xC6, 0x3F, 0x80, 0xC7,
        0x4B, 0x11, 0x26, 0x3E, 0x42, 0x1E, 0xBF, 0x84, 0x86, 0xA4, 0xE3, 0x98, 0xD0, 0xDB, 0xC0,
        0x9F, 0xA7, 0xD4, 0xF6, 0x2C, 0xCD, 0xB3, 0x09, 0xF3, 0xAE, 0xA8, 0x1F, 0x09, 0x00, 0x00,
        0x00, 0x01, 0x21, 0x7D, 0xA6, 0xC6, 0xB3, 0xE1, 0x9F, 0x18, 0x01, 0x00, 0x00, 0x00, 0x04,
        0x00, 0x00, 0x00, 0xCA, 0xFE, 0xD0, 0x0D,
    ];

    let actual_output = SimpleSerializer::<Vec<u8>>::serialize(&input).unwrap();
    assert_eq!(expected_output, actual_output);
}

fn get_common_program() -> Script {
    Script::new(
        b"move".to_vec(),
        vec![
            TransactionArgument::String("CAFE D00D".to_string()),
            TransactionArgument::String("cafe d00d".to_string()),
        ],
    )
}

fn get_common_write_set() -> WriteSet {
    WriteSetMut::new(vec![
        (
            AccessPath::new(
                AccountAddress::new([
                    0xa7, 0x1d, 0x76, 0xfa, 0xa2, 0xd2, 0xd5, 0xc3, 0x22, 0x4e, 0xc3, 0xd4, 0x1d,
                    0xeb, 0x29, 0x39, 0x73, 0x56, 0x4a, 0x79, 0x1e, 0x55, 0xc6, 0x78, 0x2b, 0xa7,
                    0x6c, 0x2b, 0xf0, 0x49, 0x5f, 0x9a,
                ]),
                vec![
                    0x01, 0x21, 0x7d, 0xa6, 0xc6, 0xb3, 0xe1, 0x9f, 0x18, 0x25, 0xcf, 0xb2, 0x67,
                    0x6d, 0xae, 0xcc, 0xe3, 0xbf, 0x3d, 0xe0, 0x3c, 0xf2, 0x66, 0x47, 0xc7, 0x8d,
                    0xf0, 0x0b, 0x37, 0x1b, 0x25, 0xcc, 0x97,
                ],
            ),
            WriteOp::Deletion,
        ),
        (
            AccessPath::new(
                AccountAddress::new([
                    0xc4, 0xc6, 0x3f, 0x80, 0xc7, 0x4b, 0x11, 0x26, 0x3e, 0x42, 0x1e, 0xbf, 0x84,
                    0x86, 0xa4, 0xe3, 0x98, 0xd0, 0xdb, 0xc0, 0x9f, 0xa7, 0xd4, 0xf6, 0x2c, 0xcd,
                    0xb3, 0x09, 0xf3, 0xae, 0xa8, 0x1f,
                ]),
                vec![0x01, 0x21, 0x7d, 0xa6, 0xc6, 0xb3, 0xe1, 0x9f, 0x18],
            ),
            WriteOp::Value(vec![0xca, 0xfe, 0xd0, 0x0d]),
        ),
    ])
    .freeze()
    .unwrap()
}
