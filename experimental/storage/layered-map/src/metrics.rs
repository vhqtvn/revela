// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_gauge_vec, HistogramVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

pub static TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_layered_map_timer_seconds",
        "Various timers for performance analysis.",
        &["use_case", "event"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
    )
    .unwrap()
});

pub static LAYER: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_layered_map_layer",
        "Various generations to help debugging.",
        &["use_case", "event"],
    )
    .unwrap()
});
