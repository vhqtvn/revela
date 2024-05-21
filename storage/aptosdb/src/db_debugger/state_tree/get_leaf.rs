// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_debugger::common::DbDir, schema::jellyfish_merkle_node::JellyfishMerkleNodeSchema};
use aptos_jellyfish_merkle::{iterator::JellyfishMerkleIterator, node_type::NodeKey};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{transaction::Version};
use clap::Parser;
use owo_colors::OwoColorize;
use std::sync::Arc;
use aptos_crypto::hash::CryptoHash;

#[derive(Parser)]
#[clap(about = "Print leaf info for given leaf index in a snapshot")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long)]
    before_version: Version,

    #[clap(long)]
    leaf_index: usize,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        ensure!(self.before_version > 0, "version must be greater than 0.");
        println!(
            "{}",
            format!(
                "* Get full path from the latest root strictly before version {} to leaf #{}. \n",
                self.before_version, self.leaf_index,
            )
            .yellow()
        );

        let db = Arc::new(self.db_dir.open_state_merkle_db()?);

        let root_version = {
            let mut iter = db
                .metadata_db()
                .rev_iter::<JellyfishMerkleNodeSchema>(Default::default())?;
            iter.seek_for_prev(&NodeKey::new_empty_path(self.before_version - 1))?;
            iter.next().transpose()?.unwrap().0.version()
        };
        let total_leaves = db.get_leaf_count(root_version)?;
        println!(
            "{}",
            format!("* Root version: {root_version}. Total leaves: {total_leaves}. \n").yellow()
        );
        ensure!(self.leaf_index < total_leaves, "leaf index out of range.");

        let (key_hash, (state_key, leaf_version)) =
            JellyfishMerkleIterator::new_by_index(db, root_version, self.leaf_index)?
                .next()
                .transpose()?
                .unwrap();
        assert_eq!(key_hash, state_key.hash());

        let serialized = hex::encode(bcs::to_bytes(&state_key).unwrap());
        println!("           state key: {:?}\n", state_key);
        println!("             version: {:?}\n", leaf_version);
        println!("          serialized: {}\n", serialized);
        println!("    full nibble path: {:x}", key_hash);

        Ok(())
    }
}
