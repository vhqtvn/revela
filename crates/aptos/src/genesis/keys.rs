// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::OptionalPoolAddressArgs;
use crate::common::utils::{create_dir_if_not_exist, current_dir, dir_default_to_current};
use crate::genesis::git::{LAYOUT_FILE, OPERATOR_FILE, OWNER_FILE};
use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions, RngArgs},
        utils::{check_if_file_exists, read_from_file, write_to_user_only_file},
    },
    genesis::git::{from_yaml, to_yaml, GitOptions},
    CliCommand,
};
use aptos_genesis::config::{Layout, OperatorConfiguration, OwnerConfiguration};
use aptos_genesis::keys::PublicIdentity;
use aptos_genesis::{config::HostAndPort, keys::generate_key_objects};
use async_trait::async_trait;
use clap::Parser;
use std::path::{Path, PathBuf};

const PRIVATE_KEYS_FILE: &str = "private-keys.yaml";
pub const PUBLIC_KEYS_FILE: &str = "public-keys.yaml";
const VALIDATOR_FILE: &str = "validator-identity.yaml";
const VFN_FILE: &str = "validator-full-node-identity.yaml";

/// Generate account key, consensus key, and network key for a validator
#[derive(Parser)]
pub struct GenerateKeys {
    #[clap(flatten)]
    pub(crate) pool_address_args: OptionalPoolAddressArgs,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub rng_args: RngArgs,
    /// Output directory for the key files
    #[clap(long, parse(from_os_str))]
    pub(crate) output_dir: Option<PathBuf>,
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for GenerateKeys {
    fn command_name(&self) -> &'static str {
        "GenerateKeys"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let output_dir = dir_default_to_current(self.output_dir.clone())?;

        let private_keys_file = output_dir.join(PRIVATE_KEYS_FILE);
        let public_keys_file = output_dir.join(PUBLIC_KEYS_FILE);
        let validator_file = output_dir.join(VALIDATOR_FILE);
        let vfn_file = output_dir.join(VFN_FILE);
        check_if_file_exists(private_keys_file.as_path(), self.prompt_options)?;
        check_if_file_exists(public_keys_file.as_path(), self.prompt_options)?;
        check_if_file_exists(validator_file.as_path(), self.prompt_options)?;
        check_if_file_exists(vfn_file.as_path(), self.prompt_options)?;

        let mut key_generator = self.rng_args.key_generator()?;
        let (mut validator_blob, mut vfn_blob, private_identity, public_identity) =
            generate_key_objects(&mut key_generator)?;

        // Allow for the owner to be different than the operator
        if let Some(pool_address) = self.pool_address_args.pool_address() {
            validator_blob.account_address = Some(pool_address);
            vfn_blob.account_address = Some(pool_address);
        }

        // Create the directory if it doesn't exist
        create_dir_if_not_exist(output_dir.as_path())?;

        write_to_user_only_file(
            private_keys_file.as_path(),
            PRIVATE_KEYS_FILE,
            to_yaml(&private_identity)?.as_bytes(),
        )?;
        write_to_user_only_file(
            public_keys_file.as_path(),
            PUBLIC_KEYS_FILE,
            to_yaml(&public_identity)?.as_bytes(),
        )?;
        write_to_user_only_file(
            validator_file.as_path(),
            VALIDATOR_FILE,
            to_yaml(&validator_blob)?.as_bytes(),
        )?;
        write_to_user_only_file(vfn_file.as_path(), VFN_FILE, to_yaml(&vfn_blob)?.as_bytes())?;
        Ok(vec![
            public_keys_file,
            private_keys_file,
            validator_file,
            vfn_file,
        ])
    }
}

/// Set validator configuration for a single validator in the git repository
#[derive(Parser)]
pub struct SetValidatorConfiguration {
    /// Name of validator
    #[clap(long)]
    pub(crate) username: String,
    #[clap(flatten)]
    pub(crate) git_options: GitOptions,
    /// Host and port pair for the validator e.g. 127.0.0.1:6180 or aptoslabs.com:6180
    #[clap(long)]
    pub(crate) validator_host: HostAndPort,
    /// Host and port pair for the fullnode e.g. 127.0.0.1:6180 or aptoslabs.com:6180
    #[clap(long)]
    pub(crate) full_node_host: Option<HostAndPort>,
    /// Stake amount for stake distribution
    #[clap(long, default_value_t = 1)]
    pub(crate) stake_amount: u64,
    /// Path to private identity generated from GenerateKeys
    #[clap(long, parse(from_os_str))]
    pub(crate) owner_public_identity_file: Option<PathBuf>,
    /// Path to operator public identity, defaults to owner identity
    #[clap(long, parse(from_os_str))]
    pub(crate) operator_public_identity_file: Option<PathBuf>,
    /// Path to voter public identity, defaults to owner identity
    #[clap(long, parse(from_os_str))]
    pub(crate) voter_public_identity_file: Option<PathBuf>,
}

#[async_trait]
impl CliCommand<()> for SetValidatorConfiguration {
    fn command_name(&self) -> &'static str {
        "SetValidatorConfiguration"
    }

    async fn execute(self) -> CliTypedResult<()> {
        // Load owner
        let owner_keys_file = if let Some(owner_keys_file) = self.owner_public_identity_file {
            owner_keys_file
        } else {
            current_dir()?.join(PUBLIC_KEYS_FILE)
        };
        let owner_identity = read_public_identity_file(owner_keys_file.as_path())?;

        // Load voter
        let voter_identity = if let Some(voter_keys_file) = self.voter_public_identity_file {
            read_public_identity_file(voter_keys_file.as_path())?
        } else {
            owner_identity.clone()
        };

        // Load operator
        let (operator_identity, operator_keys_file) =
            if let Some(operator_keys_file) = self.operator_public_identity_file {
                (
                    read_public_identity_file(operator_keys_file.as_path())?,
                    operator_keys_file,
                )
            } else {
                (owner_identity.clone(), owner_keys_file)
            };

        // Extract the possible optional fields
        let consensus_public_key =
            if let Some(consensus_public_key) = operator_identity.consensus_public_key {
                consensus_public_key
            } else {
                return Err(CliError::CommandArgumentError(format!(
                    "Failed to read consensus public key from public identity file {}",
                    operator_keys_file.display()
                )));
            };

        let validator_network_public_key = if let Some(validator_network_public_key) =
            operator_identity.validator_network_public_key
        {
            validator_network_public_key
        } else {
            return Err(CliError::CommandArgumentError(format!(
                "Failed to read validator network public key from public identity file {}",
                operator_keys_file.display()
            )));
        };

        let consensus_proof_of_possession = if let Some(consensus_proof_of_possession) =
            operator_identity.consensus_proof_of_possession
        {
            consensus_proof_of_possession
        } else {
            return Err(CliError::CommandArgumentError(format!(
                "Failed to read consensus proof of possession from public identity file {}",
                operator_keys_file.display()
            )));
        };

        // Only add the public key if there is a full node
        let full_node_network_public_key = if self.full_node_host.is_some() {
            operator_identity.validator_network_public_key
        } else {
            None
        };

        // Build operator configuration file
        let operator_config = OperatorConfiguration {
            operator_account_address: operator_identity.account_address,
            operator_account_public_key: operator_identity.account_public_key.clone(),
            consensus_public_key,
            consensus_proof_of_possession,
            validator_network_public_key,
            validator_host: self.validator_host,
            full_node_network_public_key,
            full_node_host: self.full_node_host,
        };

        let owner_config = OwnerConfiguration {
            owner_account_address: owner_identity.account_address,
            owner_account_public_key: owner_identity.account_public_key,
            voter_account_address: voter_identity.account_address,
            voter_account_public_key: voter_identity.account_public_key,
            operator_account_address: operator_identity.account_address,
            operator_account_public_key: operator_identity.account_public_key,
            stake_amount: self.stake_amount,
        };

        let directory = PathBuf::from(&self.username);
        let operator_file = directory.join(OPERATOR_FILE);
        let owner_file = directory.join(OWNER_FILE);

        let git_client = self.git_options.get_client()?;
        git_client.put(operator_file.as_path(), &operator_config)?;
        git_client.put(owner_file.as_path(), &owner_config)
    }
}

pub fn read_public_identity_file(public_identity_file: &Path) -> CliTypedResult<PublicIdentity> {
    let bytes = read_from_file(public_identity_file)?;
    from_yaml(&String::from_utf8(bytes).map_err(CliError::from)?)
}

/// Generate a Layout template file with empty values
///
/// This will generate a layout template file for genesis with some default values.  To start a
/// new chain, these defaults should be carefully thought through and chosen.
#[derive(Parser)]
pub struct GenerateLayoutTemplate {
    /// Path of the output layout template
    #[clap(long, parse(from_os_str), default_value = LAYOUT_FILE)]
    pub(crate) output_file: PathBuf,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<()> for GenerateLayoutTemplate {
    fn command_name(&self) -> &'static str {
        "GenerateLayoutTemplate"
    }

    async fn execute(self) -> CliTypedResult<()> {
        check_if_file_exists(self.output_file.as_path(), self.prompt_options)?;
        let layout = Layout::default();

        write_to_user_only_file(
            self.output_file.as_path(),
            &self.output_file.display().to_string(),
            to_yaml(&layout)?.as_bytes(),
        )
    }
}
