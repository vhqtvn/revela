// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod checkpoint;
mod common;
mod ledger;
mod state_tree;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub enum Cmd {
    #[clap(subcommand)]
    StateTree(state_tree::Cmd),

    Checkpoint(checkpoint::Cmd),

    #[clap(subcommand)]
    Ledger(ledger::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Cmd::StateTree(cmd) => cmd.run(),
            Cmd::Checkpoint(cmd) => cmd.run(),
            Cmd::Ledger(cmd) => cmd.run(),
        }
    }
}
