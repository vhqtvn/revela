// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};
use structopt::StructOpt;

use aptos_types::on_chain_config::{Version, DIEM_MAX_KNOWN_VERSION};
use framework::dpn_files;
use move_model::run_model_builder;
use move_stackless_bytecode_interpreter::{
    concrete::settings::InterpreterSettings, StacklessBytecodeInterpreter,
};

use aptos_e2e_tests_replay::{self, ReplayFlags};

#[derive(StructOpt)]
struct ReplayArgs {
    /// Trace files
    #[structopt(short = "t", long = "trace")]
    trace_files: Vec<String>,

    /// Diem selector, if set, replay traces executed in that version instead of the latest version
    #[structopt(short = "d", long = "diem")]
    diem_version: Option<u64>,

    /// Trace filters, if specified, only replay traces that passes the filter
    #[structopt(short = "f", long = "filter")]
    filters: Vec<String>,

    /// Maximum number of steps per trace to replay
    #[structopt(short = "l", long = "limit")]
    step_limit: Option<usize>,

    /// Cross check the stackless VM against the Move VM
    #[structopt(short = "x", long = "xrun")]
    xrun: bool,

    /// Cross check the stackless VM against the Move VM without invoking the expression checker
    #[structopt(short = "X", long = "xrun-shallow", conflicts_with = "xrun")]
    xrun_shallow: bool,

    /// Verbose mode
    #[structopt(short = "v", long = "verbose")]
    verbose: Option<u64>,

    /// Warning mode
    #[structopt(short = "w", long = "warning")]
    warning: Option<u64>,
}

pub fn main() -> Result<()> {
    let args = ReplayArgs::from_args();
    let mut filters = BTreeMap::new();
    for item in args.filters {
        let tokens: Vec<&str> = item.split("::").collect();
        if tokens.len() == 1 {
            filters
                .entry(tokens[0].to_string())
                .or_insert_with(BTreeSet::new);
        } else if tokens.len() == 2 {
            let step: usize = tokens[1].parse()?;
            filters
                .entry(tokens[0].to_string())
                .or_insert_with(BTreeSet::new)
                .insert(step);
        } else {
            bail!("Invalid filter: {}", item);
        }
    }

    let flags = ReplayFlags {
        diem_version: args
            .diem_version
            .map_or(DIEM_MAX_KNOWN_VERSION, |v| Version { major: v }),
        filters,
        step_limit: args.step_limit.unwrap_or(usize::MAX),
        xrun: args.xrun,
        verbose_trace_meta: args.verbose.map_or(false, |level| level > 0),
        verbose_trace_step: args.verbose.map_or(false, |level| level > 1),
        verbose_trace_xrun: args.verbose.map_or(false, |level| level > 2),
        verbose_vm: args.verbose.map_or(false, |level| level > 3),
        warning: args.warning.map_or(false, |level| level > 0),
    };

    let mut settings = if flags.verbose_vm {
        InterpreterSettings::verbose_default()
    } else {
        InterpreterSettings::default()
    };
    if args.xrun_shallow {
        settings.no_expr_check = true;
    }

    let env = run_model_builder(&dpn_files(), &[])?;
    let interpreter = StacklessBytecodeInterpreter::new(&env, None, settings);
    for trace in args.trace_files {
        aptos_e2e_tests_replay::replay(trace, &interpreter, &flags)?;
    }
    Ok(())
}
