// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_debugger::ShardingConfig, ledger_db::LedgerDb, state_merkle_db::StateMerkleDb,
    STATE_MERKLE_DB_NAME,
};
use anyhow::Result;
use aptos_config::config::RocksdbConfigs;
use aptos_types::nibble::{nibble_path::NibblePath, Nibble};
use clap::Parser;
use core::default::Default;
use std::path::{Path, PathBuf};

pub const PAGE_SIZE: usize = 10;

#[derive(Parser, Clone)]
pub struct DbDir {
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    #[clap(flatten)]
    pub sharding_config: ShardingConfig,
}

impl DbDir {
    pub fn open_state_merkle_db(&self) -> Result<StateMerkleDb> {
        StateMerkleDb::new(
            self.db_dir.join(STATE_MERKLE_DB_NAME).as_path(),
            RocksdbConfigs {
                enable_storage_sharding: self.sharding_config.enable_storage_sharding,
                ..Default::default()
            },
            false,
            0,
        )
    }

    pub fn open_ledger_db(&self) -> Result<LedgerDb> {
        LedgerDb::new(
            self.db_dir.as_path(),
            RocksdbConfigs {
                enable_storage_sharding: self.sharding_config.enable_storage_sharding,
                ..Default::default()
            },
            true,
        )
    }
}

impl AsRef<Path> for DbDir {
    fn as_ref(&self) -> &Path {
        self.db_dir.as_path()
    }
}

pub fn parse_nibble_path(src: &str) -> Result<NibblePath> {
    src.chars()
        .map(|c| Ok(Nibble::from(u8::from_str_radix(&c.to_string(), 16)?)))
        .collect()
}
