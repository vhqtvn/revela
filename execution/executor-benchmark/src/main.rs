// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_block_partitioner::{
    pre_partition::{
        connected_component::config::ConnectedComponentPartitionerConfig,
        default_pre_partitioner_config, uniform_partitioner::config::UniformPartitionerConfig,
        PrePartitionerConfig,
    },
    v2::config::PartitionerV2Config,
};
use aptos_config::config::{
    EpochSnapshotPrunerConfig, LedgerPrunerConfig, PrunerConfig, StateMerklePrunerConfig,
};
use aptos_executor::block_executor::TransactionBlockExecutor;
use aptos_executor_benchmark::{native_executor::NativeExecutor, pipeline::PipelineConfig};
use aptos_experimental_ptx_executor::PtxBlockExecutor;
#[cfg(target_os = "linux")]
use aptos_experimental_runtimes::thread_manager::{ThreadConfigStrategy, ThreadManagerBuilder};
use aptos_metrics_core::{register_int_gauge, IntGauge};
use aptos_profiler::{ProfilerConfig, ProfilerHandler};
use aptos_push_metrics::MetricsPusher;
use aptos_transaction_generator_lib::args::TransactionTypeArg;
use aptos_vm::AptosVM;
use clap::{ArgGroup, Parser, Subcommand};
use once_cell::sync::Lazy;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// This is needed for filters on the Grafana dashboard working as its used to populate the filter
/// variables.
pub static START_TIME: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("node_process_start_time", "Start time").unwrap());

#[derive(Debug, Parser)]
struct PrunerOpt {
    #[clap(long)]
    enable_state_pruner: bool,

    #[clap(long)]
    enable_epoch_snapshot_pruner: bool,

    #[clap(long)]
    enable_ledger_pruner: bool,

    #[clap(long, default_value_t = 100000)]
    state_prune_window: u64,

    #[clap(long, default_value_t = 100000)]
    epoch_snapshot_prune_window: u64,

    #[clap(long, default_value_t = 100000)]
    ledger_prune_window: u64,

    #[clap(long, default_value_t = 500)]
    ledger_pruning_batch_size: usize,

    #[clap(long, default_value_t = 500)]
    state_pruning_batch_size: usize,

    #[clap(long, default_value_t = 500)]
    epoch_snapshot_pruning_batch_size: usize,
}

impl PrunerOpt {
    fn pruner_config(&self) -> PrunerConfig {
        PrunerConfig {
            state_merkle_pruner_config: StateMerklePrunerConfig {
                enable: self.enable_state_pruner,
                prune_window: self.state_prune_window,
                batch_size: self.state_pruning_batch_size,
            },
            epoch_snapshot_pruner_config: EpochSnapshotPrunerConfig {
                enable: self.enable_epoch_snapshot_pruner,
                prune_window: self.epoch_snapshot_prune_window,
                batch_size: self.epoch_snapshot_pruning_batch_size,
            },
            ledger_pruner_config: LedgerPrunerConfig {
                enable: self.enable_ledger_pruner,
                prune_window: self.ledger_prune_window,
                batch_size: self.ledger_pruning_batch_size,
                user_pruning_window_offset: 0,
            },
        }
    }
}

#[derive(Debug, Parser)]
pub struct PipelineOpt {
    #[clap(long)]
    generate_then_execute: bool,
    #[clap(long)]
    split_stages: bool,
    #[clap(long)]
    skip_commit: bool,
    #[clap(long)]
    allow_discards: bool,
    #[clap(long)]
    allow_aborts: bool,
    #[clap(long, default_value = "1")]
    num_executor_shards: usize,
    #[clap(long)]
    use_global_executor: bool,
    #[clap(long, default_value = "4")]
    num_generator_workers: usize,
    #[clap(long, default_value = "4")]
    max_partitioning_rounds: usize,
    #[clap(long, default_value = "0.90")]
    partitioner_cross_shard_dep_avoid_threshold: f32,
    #[clap(long)]
    partitioner_version: Option<String>,
    #[clap(long)]
    pre_partitioner: Option<String>,
    #[clap(long, default_value = "2.0")]
    load_imbalance_tolerance: f32,
    #[clap(long, default_value = "8")]
    partitioner_v2_num_threads: usize,
    #[clap(long, default_value = "64")]
    partitioner_v2_dashmap_num_shards: usize,
}

impl PipelineOpt {
    fn pipeline_config(&self) -> PipelineConfig {
        PipelineConfig {
            delay_execution_start: self.generate_then_execute,
            split_stages: self.split_stages,
            skip_commit: self.skip_commit,
            allow_discards: self.allow_discards,
            allow_aborts: self.allow_aborts,
            num_executor_shards: self.num_executor_shards,
            use_global_executor: self.use_global_executor,
            num_generator_workers: self.num_generator_workers,
            partitioner_config: self.partitioner_config(),
        }
    }

    fn pre_partitioner_config(&self) -> Box<dyn PrePartitionerConfig> {
        match self.pre_partitioner.as_deref() {
            None => default_pre_partitioner_config(),
            Some("uniform") => Box::new(UniformPartitionerConfig {}),
            Some("connected-component") => Box::new(ConnectedComponentPartitionerConfig {
                load_imbalance_tolerance: self.load_imbalance_tolerance,
            }),
            _ => panic!("Unknown PrePartitioner: {:?}", self.pre_partitioner),
        }
    }

    fn partitioner_config(&self) -> PartitionerV2Config {
        match self.partitioner_version.as_deref() {
            Some("v2") => PartitionerV2Config {
                num_threads: self.partitioner_v2_num_threads,
                max_partitioning_rounds: self.max_partitioning_rounds,
                cross_shard_dep_avoid_threshold: self.partitioner_cross_shard_dep_avoid_threshold,
                dashmap_num_shards: self.partitioner_v2_dashmap_num_shards,
                partition_last_round: !self.use_global_executor,
                pre_partitioner_config: self.pre_partitioner_config(),
            },
            None => PartitionerV2Config::default(),
            _ => panic!(
                "Unknown partitioner version: {:?}",
                self.partitioner_version
            ),
        }
    }
}

#[derive(Parser, Debug)]
struct ProfilerOpt {
    #[clap(long)]
    cpu_profiling: bool,

    #[clap(long)]
    memory_profiling: bool,
}

#[derive(Parser, Debug)]
#[clap(group(
    ArgGroup::new("vm_selection")
    .args(&["use_native_executor", "use_ptx_executor"]),
))]
pub struct VmSelectionOpt {
    #[clap(long)]
    use_native_executor: bool,

    #[clap(long)]
    use_ptx_executor: bool,
}

#[derive(Parser, Debug)]
struct Opt {
    #[clap(long, default_value_t = 10000)]
    block_size: usize,

    #[clap(long, default_value_t = 5)]
    transactions_per_sender: usize,

    /// 0 implies random TX generation; if non-zero, then 'transactions_per_sender is ignored
    /// 'connected_tx_grps' should be less than 'block_size'
    #[clap(long, default_value_t = 0)]
    connected_tx_grps: usize,

    #[clap(long)]
    shuffle_connected_txns: bool,

    #[clap(long, conflicts_with_all = &["connected_tx_grps", "transactions_per_sender"])]
    hotspot_probability: Option<f32>,

    #[clap(
        long,
        help = "Number of threads to use for execution. Generally replaces --concurrency-level flag (directly for default case, and as a total across all shards for sharded case)"
    )]
    execution_threads: Option<usize>,

    #[clap(flatten)]
    pruner_opt: PrunerOpt,

    #[clap(long)]
    enable_storage_sharding: bool,

    #[clap(flatten)]
    pipeline_opt: PipelineOpt,

    #[clap(subcommand)]
    cmd: Command,

    /// Verify sequence number of all the accounts after execution finishes
    #[clap(long)]
    verify_sequence_numbers: bool,

    #[clap(flatten)]
    vm_selection_opt: VmSelectionOpt,

    #[clap(flatten)]
    profiler_opt: ProfilerOpt,
}

impl Opt {
    fn execution_threads(&self) -> usize {
        match self.execution_threads {
            None => {
                let cores = num_cpus::get();
                println!("\nExecution threads defaults to number of cores: {}", cores,);
                cores
            },
            Some(threads) => threads,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    CreateDb {
        #[clap(long, value_parser)]
        data_dir: PathBuf,

        #[clap(long, default_value_t = 1000000)]
        num_accounts: usize,

        #[clap(long, default_value_t = 10000000000)]
        init_account_balance: u64,
    },
    RunExecutor {
        /// number of transfer blocks to run
        #[clap(long, default_value_t = 1000)]
        blocks: usize,

        #[clap(long, default_value_t = 1000000)]
        main_signer_accounts: usize,

        #[clap(long, default_value_t = 0)]
        additional_dst_pool_accounts: usize,

        /// Workload (transaction type). Uses raw coin transfer if not set,
        /// and if set uses transaction-generator-lib to generate it
        #[clap(
            long,
            value_enum,
            num_args = 0..,
            ignore_case = true
        )]
        transaction_type: Vec<TransactionTypeArg>,

        #[clap(long, num_args = 0..)]
        transaction_weights: Vec<usize>,

        #[clap(long, default_value_t = 1)]
        module_working_set_size: usize,

        #[clap(long, value_parser)]
        data_dir: PathBuf,

        #[clap(long, value_parser)]
        checkpoint_dir: PathBuf,
    },
    AddAccounts {
        #[clap(long, value_parser)]
        data_dir: PathBuf,

        #[clap(long, value_parser)]
        checkpoint_dir: PathBuf,

        #[clap(long, default_value_t = 1000000)]
        num_new_accounts: usize,

        #[clap(long, default_value_t = 1000000)]
        init_account_balance: u64,
    },
}

fn run<E>(opt: Opt)
where
    E: TransactionBlockExecutor + 'static,
{
    match opt.cmd {
        Command::CreateDb {
            data_dir,
            num_accounts,
            init_account_balance,
        } => {
            aptos_executor_benchmark::db_generator::create_db_with_accounts::<E>(
                num_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
                opt.enable_storage_sharding,
                opt.pipeline_opt.pipeline_config(),
            );
        },
        Command::RunExecutor {
            blocks,
            main_signer_accounts,
            additional_dst_pool_accounts,
            transaction_type,
            transaction_weights,
            module_working_set_size,
            data_dir,
            checkpoint_dir,
        } => {
            let transaction_mix = if transaction_type.is_empty() {
                None
            } else {
                let mix_per_phase = TransactionTypeArg::args_to_transaction_mix_per_phase(
                    &transaction_type,
                    &transaction_weights,
                    &[],
                    module_working_set_size,
                    false,
                );
                assert!(mix_per_phase.len() == 1);
                Some(mix_per_phase[0].clone())
            };

            if let Some(hotspot_probability) = opt.hotspot_probability {
                if !(0.5..1.0).contains(&hotspot_probability) {
                    panic!("Parameter hotspot-probability has to a decimal number in [0.5, 1.0).");
                }
            }

            aptos_executor_benchmark::run_benchmark::<E>(
                opt.block_size,
                blocks,
                transaction_mix,
                opt.transactions_per_sender,
                opt.connected_tx_grps,
                opt.shuffle_connected_txns,
                opt.hotspot_probability,
                main_signer_accounts,
                additional_dst_pool_accounts,
                data_dir,
                checkpoint_dir,
                opt.verify_sequence_numbers,
                opt.pruner_opt.pruner_config(),
                opt.enable_storage_sharding,
                opt.pipeline_opt.pipeline_config(),
            );
        },
        Command::AddAccounts {
            data_dir,
            checkpoint_dir,
            num_new_accounts,
            init_account_balance,
        } => {
            aptos_executor_benchmark::add_accounts::<E>(
                num_new_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                checkpoint_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
                opt.enable_storage_sharding,
                opt.pipeline_opt.pipeline_config(),
            );
        },
    }
}

fn main() {
    let opt = Opt::parse();
    aptos_logger::Logger::new().init();
    START_TIME.set(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );
    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");

    aptos_node_resource_metrics::register_node_metrics_collector();
    let _mp = MetricsPusher::start_for_local_run("executor-benchmark");

    let execution_threads = opt.execution_threads();
    let execution_shards = opt.pipeline_opt.num_executor_shards;
    assert!(
        execution_threads % execution_shards == 0,
        "Execution threads ({}) must be divisible by the number of execution shards ({}).",
        execution_threads,
        execution_shards
    );
    let execution_threads_per_shard = execution_threads / execution_shards;

    AptosVM::set_num_shards_once(execution_shards);
    AptosVM::set_concurrency_level_once(execution_threads_per_shard);
    NativeExecutor::set_concurrency_level_once(execution_threads_per_shard);

    let config = ProfilerConfig::new_with_defaults();
    let handler = ProfilerHandler::new(config);

    let cpu_profiling = opt.profiler_opt.cpu_profiling;
    let memory_profiling = opt.profiler_opt.memory_profiling;

    let mut cpu_profiler = handler.get_cpu_profiler();
    let mut memory_profiler = handler.get_mem_profiler();

    if cpu_profiling {
        let _cpu_start = cpu_profiler.start_profiling();
    }
    if memory_profiling {
        let _mem_start = memory_profiler.start_profiling();
    }

    if opt.vm_selection_opt.use_native_executor {
        run::<NativeExecutor>(opt);
    } else if opt.vm_selection_opt.use_ptx_executor {
        #[cfg(target_os = "linux")]
        ThreadManagerBuilder::set_thread_config_strategy(ThreadConfigStrategy::ThreadsPriority(48));
        run::<PtxBlockExecutor>(opt);
    } else {
        run::<AptosVM>(opt);
    }

    if cpu_profiling {
        let _cpu_end = cpu_profiler.end_profiling("");
    }
    if memory_profiling {
        let _mem_end = memory_profiler.end_profiling("./target/release/aptos-executor-benchmark");
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Opt::command().debug_assert()
}
