// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator_v2::{initialize, AggV2TestHarness, AggregatorLocation, ElementType, UseType},
    assert_abort, assert_success,
    tests::common,
    BlockSplit, SUCCESS,
};
use aptos_framework::natives::aggregator_natives::aggregator_v2::{
    EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED, EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
};
use aptos_language_e2e_tests::executor::ExecutorMode;
use proptest::prelude::*;

const EAGGREGATOR_OVERFLOW: u64 = 0x02_0001;
const EAGGREGATOR_UNDERFLOW: u64 = 0x02_0002;

const DEFAULT_EXECUTOR_MODE: ExecutorMode = ExecutorMode::SequentialOnly;

fn setup(
    executor_mode: ExecutorMode,
    aggregator_execution_enabled: bool,
    txns: usize,
) -> AggV2TestHarness {
    initialize(
        common::test_dir_path("aggregator_v2.data/pack"),
        executor_mode,
        aggregator_execution_enabled,
        txns,
    )
}

#[cfg(test)]
mod test_cases {
    use super::*;
    use test_case::test_case;

    #[test_case(true)]
    #[test_case(false)]
    fn test_copy_snapshot(execution_enabled: bool) {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, execution_enabled, 1);
        let txn = h.verify_copy_snapshot();
        assert_abort!(h.harness.run(txn), EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED);
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_copy_string_snapshot(execution_enabled: bool) {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, execution_enabled, 1);
        let txn = h.verify_copy_string_snapshot();
        assert_abort!(h.harness.run(txn), EAGGREGATOR_FUNCTION_NOT_YET_SUPPORTED);
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_snapshot_concat(execution_enabled: bool) {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, execution_enabled, 1);
        let txn = h.verify_string_concat();
        assert_success!(h.harness.run(txn));
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_string_snapshot_concat(execution_enabled: bool) {
        let mut h = setup(DEFAULT_EXECUTOR_MODE, execution_enabled, 1);
        let txn = h.verify_string_snapshot_concat();
        assert_abort!(h.harness.run(txn), EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE);
    }

    // This tests uses multuple blocks, so requires exchange to be done to work.
    // #[test_case(true)]
    #[test_case(false)]
    fn test_aggregators_e2e(execution_enabled: bool) {
        println!("Testing test_aggregators_e2e {:?}", execution_enabled);
        let element_type = ElementType::U64;
        let use_type = UseType::UseTableType;

        let mut h = setup(DEFAULT_EXECUTOR_MODE, execution_enabled, 100);

        let init_txn = h.init(None, use_type, element_type, true);
        h.run_block_in_parts_and_check(BlockSplit::Whole, vec![(SUCCESS, init_txn)]);

        let addr = *h.account.address();
        let loc = |i| AggregatorLocation::new(addr, element_type, use_type, i);

        let block_size = 30;

        // Create many aggregators with deterministic limit.
        let txns = (0..block_size)
            .map(|i| (SUCCESS, h.new(&loc(i), (i as u128) * 100000)))
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

        // All transactions in block must fail, so values of aggregators are still 0.
        let failed_txns = (0..block_size)
            .map(|i| match i % 2 {
                0 => (
                    EAGGREGATOR_OVERFLOW,
                    h.materialize_and_add(&loc(i), (i as u128) * 100000 + 1),
                ),
                _ => (
                    EAGGREGATOR_UNDERFLOW,
                    h.materialize_and_sub(&loc(i), (i as u128) * 100000 + 1),
                ),
            })
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, failed_txns);

        // Now test all operations. To do that, make sure aggregator have values large enough.
        let txns = (0..block_size)
            .map(|i| (SUCCESS, h.add(&loc(i), (i as u128) * 1000)))
            .collect();

        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

        // TODO[agg_v2](test): proptests with random transaction generator might be useful here.
        let txns = (0..block_size)
            .map(|i| match i % 4 {
                0 => (
                    SUCCESS,
                    h.sub_add(&loc(i), (i as u128) * 1000, (i as u128) * 3000),
                ),
                1 => (SUCCESS, h.materialize_and_add(&loc(i), (i as u128) * 1000)),
                2 => (SUCCESS, h.sub_and_materialize(&loc(i), (i as u128) * 1000)),
                _ => (SUCCESS, h.add(&loc(i), i as u128)),
            })
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);

        // Finally, check values.
        let txns = (0..block_size)
            .map(|i| match i % 4 {
                0 => (SUCCESS, h.check(&loc(i), (i as u128) * 3000)),
                1 => (SUCCESS, h.check(&loc(i), (i as u128) * 2000)),
                2 => (SUCCESS, h.check(&loc(i), 0)),
                _ => (SUCCESS, h.check(&loc(i), (i as u128) * 1000 + (i as u128))),
            })
            .collect();
        h.run_block_in_parts_and_check(BlockSplit::Whole, txns);
    }
}

#[allow(dead_code)]
fn arb_block_split(len: usize) -> BoxedStrategy<BlockSplit> {
    (0..3)
        .prop_flat_map(move |enum_type| {
            // making running a test with a full block likely
            if enum_type == 0 {
                Just(BlockSplit::Whole).boxed()
            } else if enum_type == 1 {
                Just(BlockSplit::SingleTxnPerBlock).boxed()
            } else {
                // First is non-empty, and not the whole block here: [1, len)
                (1usize..len)
                    .prop_flat_map(move |first| {
                        // Second is non-empty, but can finish the block: [1, len - first]
                        (Just(first), 1usize..len - first + 1)
                    })
                    .prop_map(|(first, second)| BlockSplit::SplitIntoThree {
                        first_len: first,
                        second_len: second,
                    })
                    .boxed()
            }
        })
        .boxed()
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TestEnvConfig {
    pub executor_mode: ExecutorMode,
    pub aggregator_execution_enabled: bool,
    pub block_split: BlockSplit,
}

#[allow(clippy::arc_with_non_send_sync)] // I think this is noise, don't see an issue, and tests run fine
fn arb_test_env(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        // For execution disabled, use only whole blocks and txn-per-block for block splits, as it block split shouldn't matter there.
        Just(TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            aggregator_execution_enabled: false,
            block_split: BlockSplit::Whole
        }),
        Just(TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            aggregator_execution_enabled: false,
            block_split: BlockSplit::SingleTxnPerBlock
        }),
        // Sequential execution doesn't have exchanges, so we cannot use BothComparison, nor block split
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            aggregator_execution_enabled: true,
            block_split
        }),
        // Currently, only this fails, so you can comment out all other tests, and run this one for debugging:
        // Just(TestEnvConfig {
        //     executor_mode: ExecutorMode::ParallelOnly,
        //     aggregator_execution_enabled: true,
        //     block_split: BlockSplit::SingleTxnPerBlock
        // }),
    ]
    .boxed()
}

fn arb_agg_type() -> BoxedStrategy<ElementType> {
    prop_oneof![Just(ElementType::U64), Just(ElementType::U128),].boxed()
}

// fn arb_snap_type() -> BoxedStrategy<ElementType> {
//     prop_oneof![
//         Just(ElementType::U64),
//         Just(ElementType::U128),
//         Just(ElementType::String),
//     ].boxed()
// }

fn arb_use_type() -> BoxedStrategy<UseType> {
    prop_oneof![
        Just(UseType::UseResourceType),
        Just(UseType::UseTableType),
        Just(UseType::UseResourceGroupType),
    ]
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        // We will test a few more comprehensive tests more times, and the rest even fewer.
        // when trying to stress-test, increase (to 200 or more), and disable result cache.
        cases: 10,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_lifetime(test_env in arb_test_env(14), element_type in arb_agg_type(), use_type in arb_use_type()) {
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 14);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.new(&agg_loc, 1500)),
            (SUCCESS, h.add(&agg_loc, 400)), // 400
            (SUCCESS, h.materialize(&agg_loc)),
            (SUCCESS, h.add(&agg_loc, 500)), // 900
            (SUCCESS, h.check(&agg_loc, 900)),
            (SUCCESS, h.materialize_and_add(&agg_loc, 600)), // 1500
            (SUCCESS, h.materialize_and_sub(&agg_loc, 600)), // 900
            (SUCCESS, h.check(&agg_loc, 900)),
            (SUCCESS, h.sub_add(&agg_loc, 200, 300)), // 1000
            (SUCCESS, h.check(&agg_loc, 1000)),
            // These 2 transactions fail, and should have no side-effects.
            (EAGGREGATOR_OVERFLOW, h.add_and_materialize(&agg_loc, 501)),
            (EAGGREGATOR_UNDERFLOW, h.sub_and_materialize(&agg_loc, 1001)),
            (SUCCESS, h.check(&agg_loc, 1000)),
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_multiple_aggregators_and_collocation(
        test_env in arb_test_env(24),
        element_type in arb_agg_type(),
        use_type in arb_use_type(),
        is_2_collocated in any::<bool>(),
        is_3_collocated in any::<bool>(),
    ) {
        println!("Testing test_multiple_aggregators_and_collocation {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 24);
        let acc_2 = h.harness.new_account_with_key_pair();
        let acc_3 = h.harness.new_account_with_key_pair();

        let mut idx_1 = 0;
        let agg_1_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let agg_2_loc = {
            let (cur_acc, idx_2) = if is_2_collocated { idx_1 += 1; (h.account.address(), idx_1) } else { (acc_2.address(), 0)};
            AggregatorLocation::new(*cur_acc, element_type, use_type, idx_2)
        };
        let agg_3_loc = {
            let (cur_acc, idx_3) = if is_3_collocated { idx_1 += 1; (h.account.address(), idx_1) } else { (acc_3.address(), 0)};
            AggregatorLocation::new(*cur_acc, element_type, use_type, idx_3)
        };
        println!("agg_1_loc: {:?}", agg_1_loc);
        println!("agg_2_loc: {:?}", agg_2_loc);
        println!("agg_3_loc: {:?}", agg_3_loc);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.init(Some(&acc_2), use_type, element_type, true)),
            (SUCCESS, h.init(Some(&acc_3), use_type, element_type, true)),
            (SUCCESS, h.new_add(&agg_1_loc, 10, 5)),
            (SUCCESS, h.new_add(&agg_2_loc, 10, 5)),
            (SUCCESS, h.new_add(&agg_3_loc, 10, 5)),  // 5, 5, 5
            (SUCCESS, h.add_2(&agg_1_loc, &agg_2_loc, 1, 1)), // 6, 6, 5
            (SUCCESS, h.add_2(&agg_1_loc, &agg_3_loc, 1, 1)), // 7, 6, 6
            (EAGGREGATOR_OVERFLOW, h.add(&agg_1_loc, 5)), // X
            (SUCCESS, h.add_sub(&agg_1_loc, 3, 3)), // 7, 6, 6
            (EAGGREGATOR_OVERFLOW, h.add_2(&agg_1_loc, &agg_2_loc, 3, 5)), // X
            (SUCCESS, h.add_2(&agg_1_loc, &agg_2_loc, 3, 1)), // 10, 7, 6
            (EAGGREGATOR_OVERFLOW, h.add_sub(&agg_1_loc, 3, 3)), // X
            (SUCCESS, h.sub(&agg_1_loc, 3)), // 7, 7, 6
            (SUCCESS, h.add_2(&agg_2_loc, &agg_3_loc, 2, 2)), // 7, 9, 8
            (SUCCESS, h.check(&agg_2_loc, 9)),
            (EAGGREGATOR_OVERFLOW, h.add_2(&agg_1_loc, &agg_2_loc, 1, 2)), // X
            (SUCCESS, h.add_2(&agg_2_loc, &agg_3_loc, 1, 2)), // 7, 10, 10
            (EAGGREGATOR_OVERFLOW, h.add(&agg_2_loc, 1)), // X
            (EAGGREGATOR_OVERFLOW, h.add_and_materialize(&agg_3_loc, 1)), // X
            (EAGGREGATOR_OVERFLOW, h.add_2(&agg_1_loc, &agg_2_loc, 1, 1)), // X
            (SUCCESS, h.check(&agg_1_loc, 7)),
            (SUCCESS, h.check(&agg_2_loc, 10)),
            (SUCCESS, h.check(&agg_3_loc, 10)),
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough for these
        // when trying to stress-test, increase (to 200 or more), and disable result cache.
        cases: 10,
        result_cache: prop::test_runner::basic_result_cache,
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_aggregator_underflow(test_env in arb_test_env(4)) {
        println!("Testing test_aggregator_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 4);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.new(&agg_loc, 600)),
            (SUCCESS, h.add(&agg_loc, 400)),
            // Value dropped below zero - abort with EAGGREGATOR_UNDERFLOW.
            (EAGGREGATOR_UNDERFLOW, h.sub(&agg_loc, 500))
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_underflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_underflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.new(&agg_loc, 600)),
            // Underflow on materialized value leads to abort with EAGGREGATOR_UNDERFLOW.
            (EAGGREGATOR_UNDERFLOW, h.materialize_and_sub(&agg_loc, 400)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.new_add(&agg_loc, 600, 400)),
            // Limit exceeded - abort with EAGGREGATOR_OVERFLOW.
            (EAGGREGATOR_OVERFLOW, h.add(&agg_loc, 201))
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn test_aggregator_materialize_overflow(test_env in arb_test_env(3)) {
        println!("Testing test_aggregator_materialize_overflow {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h= setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 3);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.new(&agg_loc, 399)),
            // Overflow on materialized value leads to abort with EAGGREGATOR_OVERFLOW.
            (EAGGREGATOR_OVERFLOW, h.materialize_and_add(&agg_loc, 400)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    // TODO[agg_v2](fix) Until string snapshot serialization is fixed, this cannot work.
    #[ignore]
    #[test]
    fn test_aggregator_snapshot(test_env in arb_test_env(9)) {
        println!("Testing test_aggregator_snapshot {:?}", test_env);
        let element_type = ElementType::U64;
        let use_type = UseType::UseResourceType;

        let mut h = setup(test_env.executor_mode, test_env.aggregator_execution_enabled, 9);

        let agg_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let snap_loc = AggregatorLocation::new(*h.account.address(), element_type, use_type, 0);
        let derived_snap_loc = AggregatorLocation::new(*h.account.address(), ElementType::String, use_type, 0);

        let txns = vec![
            (SUCCESS, h.init(None, use_type, element_type, true)),
            (SUCCESS, h.init(None, use_type, element_type, false)),
            (SUCCESS, h.init(None, use_type, ElementType::String, false)),
            (SUCCESS, h.new_add(&agg_loc, 400, 100)),
            (SUCCESS, h.snapshot(&agg_loc, &snap_loc)),
            (SUCCESS, h.check_snapshot(&snap_loc, 100)),
            (SUCCESS, h.read_snapshot(&agg_loc)),
            (SUCCESS, h.add_and_read_snapshot_u128(&agg_loc, 100)),
            (SUCCESS, h.concat(&snap_loc, &derived_snap_loc, "12", "13")),
            (SUCCESS, h.check_snapshot(&derived_snap_loc, 1210013)),
        ];

        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }
}
