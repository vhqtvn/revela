// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType},
};
use aptos_transaction_generator_lib::{
    publishing::{
        module_simple::{AutomaticArgs, LoopType, MultiSigConfig},
        publish_util::{Package, PackageHandler},
    },
    EntryPoints,
};
use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use std::process::exit;

pub fn execute_txn(
    executor: &mut FakeExecutor,
    account: &Account,
    sequence_number: u64,
    payload: TransactionPayload,
) {
    let sign_tx = account
        .transaction()
        .sequence_number(sequence_number)
        .max_gas_amount(2_000_000)
        .gas_unit_price(200)
        .payload(payload)
        .sign();

    let txn_output = executor.execute_transaction(sign_tx);
    executor.apply_write_set(txn_output.write_set());
    assert!(txn_output.status().status().unwrap().is_success());
}

fn execute_and_time_entry_point(
    entry_point: &EntryPoints,
    package: &Package,
    publisher_address: &AccountAddress,
    executor: &mut FakeExecutor,
    iterations: u64,
) -> u128 {
    let mut rng = StdRng::seed_from_u64(14);
    let entry_fun = entry_point
        .create_payload(
            package.get_module_id(entry_point.module_name()),
            Some(&mut rng),
            Some(publisher_address),
        )
        .into_entry_function();

    executor.exec_func_record_running_time(
        entry_fun.module(),
        entry_fun.function().as_str(),
        entry_fun.ty_args().to_vec(),
        entry_fun.args().to_vec(),
        iterations,
        match entry_point.automatic_args() {
            AutomaticArgs::None => ExecFuncTimerDynamicArgs::NoArgs,
            AutomaticArgs::Signer => ExecFuncTimerDynamicArgs::DistinctSigners,
            AutomaticArgs::SignerAndMultiSig => match entry_point.multi_sig_additional_num() {
                MultiSigConfig::Publisher => {
                    ExecFuncTimerDynamicArgs::DistinctSignersAndFixed(vec![*publisher_address])
                },
                _ => todo!(),
            },
        },
        GasMeterType::RegularGasMeter,
    )
}

const ALLOWED_REGRESSION: f32 = 0.15;
const ALLOWED_IMPROVEMENT: f32 = 0.15;
const ABSOLUTE_BUFFER_US: f32 = 2.0;

fn main() {
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let entry_points = vec![
        // too fast for the timer
        // (, EntryPoints::Nop),
        // (, EntryPoints::BytesMakeOrChange {
        //     data_length: Some(32),
        // }),
        // (, EntryPoints::IncGlobal),
        (32350, EntryPoints::Loop {
            loop_count: Some(100000),
            loop_type: LoopType::NoOp,
        }),
        (19152, EntryPoints::Loop {
            loop_count: Some(10000),
            loop_type: LoopType::Arithmetic,
        }),
        // This is a cheap bcs (serializing vec<u8>), so not representative of what BCS native call should cost.
        // (, EntryPoints::Loop { loop_count: Some(1000), loop_type: LoopType::BCS { len: 1024 }}),
        (117, EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 0,
        }),
        (6978, EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 10 * 1024,
        }),
        (1187, EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 0,
        }),
        (8676, EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 10 * 1024,
        }),
        (61, EntryPoints::InitializeVectorPicture { length: 40 }),
        (14, EntryPoints::VectorPicture { length: 40 }),
        (14, EntryPoints::VectorPictureRead { length: 40 }),
        (27303, EntryPoints::InitializeVectorPicture {
            length: 30 * 1024,
        }),
        (4507, EntryPoints::VectorPicture { length: 30 * 1024 }),
        (4469, EntryPoints::VectorPictureRead { length: 30 * 1024 }),
        (33129, EntryPoints::SmartTablePicture {
            length: 30 * 1024,
            num_points_per_txn: 200,
        }),
        (56464, EntryPoints::SmartTablePicture {
            length: 1024 * 1024,
            num_points_per_txn: 300,
        }),
        (10, EntryPoints::ResourceGroupsSenderWriteTag {
            string_length: 1024,
        }),
        (24, EntryPoints::ResourceGroupsSenderMultiChange {
            string_length: 1024,
        }),
        (257, EntryPoints::TokenV1MintAndTransferFT),
        (412, EntryPoints::TokenV1MintAndTransferNFTSequential),
        (368, EntryPoints::TokenV2AmbassadorMint { numbered: true }),
    ];

    let mut failures = Vec::new();
    let mut json_lines = Vec::new();

    println!(
        "{:>15}  {:>15}  {:>15}   entry point",
        "wall time (us)", "expected (us)", "diff(- is impr)"
    );

    for (index, (expected_time, entry_point)) in entry_points.into_iter().enumerate() {
        let publisher = executor.new_account_at(AccountAddress::random());

        let mut package_handler = PackageHandler::new(entry_point.package_name());
        let mut rng = StdRng::seed_from_u64(14);
        let package = package_handler.pick_package(&mut rng, *publisher.address());
        execute_txn(
            &mut executor,
            &publisher,
            0,
            package.publish_transaction_payload(),
        );
        if let Some(init_entry_point) = entry_point.initialize_entry_point() {
            execute_txn(
                &mut executor,
                &publisher,
                1,
                init_entry_point.create_payload(
                    package.get_module_id(init_entry_point.module_name()),
                    Some(&mut rng),
                    Some(publisher.address()),
                ),
            );
        }

        let elapsed_micros = execute_and_time_entry_point(
            &entry_point,
            &package,
            publisher.address(),
            &mut executor,
            if expected_time > 10000 {
                6
            } else if expected_time > 1000 {
                10
            } else {
                100
            },
        );
        let diff = (elapsed_micros as f32 - expected_time as f32) / (expected_time as f32) * 100.0;
        println!(
            "{:15}  {:15}  {:14.1}%   {:?}",
            elapsed_micros, expected_time, diff, entry_point
        );

        json_lines.push(json!({
            "grep": "grep_json_aptos_move_vm_perf",
            "transaction_type": format!("{:?}", entry_point),
            "wall_time_us": elapsed_micros,
            "expected_wall_time_us": expected_time,
            "test_index": index,
        }));

        if elapsed_micros as f32
            > expected_time as f32 * (1.0 + ALLOWED_REGRESSION) + ABSOLUTE_BUFFER_US
        {
            failures.push(format!(
                "Performance regression detected: {}us, expected: {}us, diff: {}%, for {:?}",
                elapsed_micros, expected_time, diff, entry_point
            ));
        } else if elapsed_micros as f32 + ABSOLUTE_BUFFER_US
            < expected_time as f32 * (1.0 - ALLOWED_IMPROVEMENT)
        {
            failures.push(format!(
                "Performance improvement detected: {}us, expected {}us, diff: {}%, for {:?}. You need to adjust expected time!",
                elapsed_micros, expected_time, diff, entry_point
            ));
        }
    }

    for line in json_lines {
        println!("{}", serde_json::to_string(&line).unwrap());
    }

    for failure in &failures {
        println!("{}", failure);
    }
    if !failures.is_empty() {
        println!("Failing, there were perf improvements or regressions.");
        exit(1);
    }

    // Assert there were no error log lines in the run.
    assert_eq!(0, aptos_logger::ERROR_LOG_COUNT.get());
}
