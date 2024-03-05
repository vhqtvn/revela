// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note: We make use of the self_update crate, but as you can see in the case of
// Revela, this can also be used to install / update other binaries.

mod aptos;
mod helpers;
mod revela;
mod tool;

use crate::common::types::CliTypedResult;
use anyhow::{anyhow, Context, Result};
pub use helpers::get_additional_binaries_dir;
pub use revela::get_revela_path;
use self_update::{update::ReleaseUpdate, version::bump_is_greater, Status};
pub use tool::UpdateTool;

/// Things that implement this trait are able to update a binary.
trait BinaryUpdater {
    /// Only used for messages we print to the user.
    fn pretty_name(&self) -> &'static str;

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo>;

    /// Build the updater from the self_update crate.
    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>>;

    /// Update the binary. Install if not present, in the case of additional binaries
    /// such as Revela.
    fn update(&self) -> CliTypedResult<String> {
        // Confirm that we need to update.
        let info = self
            .get_update_info()
            .context("Failed to check if we need to update")?;
        if !info.update_required()? {
            return Ok(format!("Already up to date (v{})", info.target_version));
        }

        // Build the updater.
        let updater = self.build_updater(&info)?;

        // Update the binary.
        let result = updater
            .update()
            .map_err(|e| anyhow!("Failed to update {}: {:#}", self.pretty_name(), e))?;

        let message = match result {
            Status::UpToDate(_) => unreachable!("We should have caught this already"),
            Status::Updated(_) => match info.current_version {
                Some(current_version) => format!(
                    "Successfully updated {} from v{} to v{}",
                    self.pretty_name(),
                    current_version,
                    info.target_version
                ),
                None => {
                    format!(
                        "Successfully installed {} v{}",
                        self.pretty_name(),
                        info.target_version
                    )
                },
            },
        };

        Ok(message)
    }
}

/// Information used to determine if an update is required. The versions given to this
/// struct should not have any prefix, it should just be the version. e.g. 2.5.0 rather
/// than aptos-cli-v2.5.0.
#[derive(Debug)]
pub struct UpdateRequiredInfo {
    pub current_version: Option<String>,
    pub target_version: String,
}

impl UpdateRequiredInfo {
    pub fn update_required(&self) -> Result<bool> {
        match self.current_version {
            Some(ref current_version) => bump_is_greater(current_version, &self.target_version)
                .context(
                    "Failed to compare current and latest CLI versions, please update manually",
                ),
            None => Ok(true),
        }
    }
}
