// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod bytecode_generator;
mod bytecode_pipeline;
mod experiments;
mod options;

use anyhow::{anyhow, bail};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, WriteColor};
pub use experiments::*;
use move_model::{model::GlobalEnv, PackageInfo};
use move_stackless_bytecode::function_target_pipeline::{
    FunctionTargetPipeline, FunctionTargetsHolder, FunctionVariant,
};
pub use options::*;
use std::path::Path;

/// Run Move compiler and print errors to stderr.
pub fn run_move_compiler_to_stderr(options: Options) -> anyhow::Result<()> {
    let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
    run_move_compiler(&mut error_writer, options)
}

/// Run move compiler and print errors to given writer.
pub fn run_move_compiler(
    error_writer: &mut impl WriteColor,
    options: Options,
) -> anyhow::Result<()> {
    // Run context check.
    let env = run_checker(options.clone())?;
    check_errors(&env, error_writer, "checking errors")?;
    // Run code generator
    let mut targets = run_bytecode_gen(&env);
    check_errors(&env, error_writer, "code generation errors")?;
    // Run transformation pipeline
    let pipeline = bytecode_pipeline(&env);
    if options.dump_bytecode {
        // Dump bytecode to files, using a basename for the individual sources derived
        // from the first input file.
        let dump_base_name = options
            .sources
            .get(0)
            .and_then(|f| {
                Path::new(f)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "dump".to_string());
        pipeline.run_with_dump(&env, &mut targets, &dump_base_name, false)
    } else {
        pipeline.run(&env, &mut targets)
    }
    bail!("bytecode lowering not implemented")
}

/// Run the type checker and return the global env (with errors if encountered). The result
/// fails not on context checking errors, but possibly on i/o errors.
pub fn run_checker(options: Options) -> anyhow::Result<GlobalEnv> {
    // Run the model builder, which performs context checking.
    let addrs = move_model::parse_addresses_from_options(options.named_address_mapping.clone())?;
    let env = move_model::run_model_builder_in_compiler_mode(
        PackageInfo {
            sources: options.sources.clone(),
            address_map: addrs.clone(),
        },
        vec![PackageInfo {
            sources: options.dependencies.clone(),
            address_map: addrs,
        }],
    )?;
    // Store options in env, for later access
    env.set_extension(options);
    Ok(env)
}

// Run the (stackless) bytecode generator. For each function which is target of the
// compilation, create an entry in the functions target holder which encapsulate info
// like the generated bytecode.
pub fn run_bytecode_gen(env: &GlobalEnv) -> FunctionTargetsHolder {
    let mut targets = FunctionTargetsHolder::default();
    for module in env.get_modules() {
        if module.is_target() {
            for fun in module.get_functions() {
                let id = fun.get_qualified_id();
                let data = bytecode_generator::generate_bytecode(env, id);
                targets.insert_target_data(&id, FunctionVariant::Baseline, data)
            }
        }
    }
    targets
}

/// Returns the bytecode processing pipeline.
pub fn bytecode_pipeline(_env: &GlobalEnv) -> FunctionTargetPipeline {
    // TODO: insert processors here as we proceed.
    // Use `env.get_extension::<Options>()` to access compiler options
    FunctionTargetPipeline::default()
}

/// Report any diags in the env to the writer and fail if there are errors.
pub fn check_errors<W: WriteColor>(
    env: &GlobalEnv,
    error_writer: &mut W,
    msg: &'static str,
) -> anyhow::Result<()> {
    let options = env.get_extension::<Options>().unwrap_or_default();
    env.report_diag(error_writer, options.report_severity());
    if env.has_errors() {
        Err(anyhow!(format!("exiting with {}", msg)))
    } else {
        Ok(())
    }
}
