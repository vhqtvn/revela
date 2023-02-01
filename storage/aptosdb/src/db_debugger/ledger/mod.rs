// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod check_range_proof;
mod check_rxn_info_hashes;

use anyhow::Result;

#[derive(clap::Subcommand)]
#[clap(about = "Check the ledger.")]
pub enum Cmd {
    CheckTransactionInfoHashes(check_rxn_info_hashes::Cmd),
    CheckRangeProof(check_range_proof::Cmd),
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        match self {
            Self::CheckTransactionInfoHashes(cmd) => cmd.run(),
            Self::CheckRangeProof(cmd) => cmd.run(),
        }
    }
}
