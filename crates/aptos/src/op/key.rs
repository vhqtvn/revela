// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            EncodingOptions, EncodingType, Error, ExtractPublicKey, KeyType,
            PrivateKeyInputOptions, SaveFile,
        },
        utils::{append_file_extension, check_if_file_exists, to_common_result, write_to_file},
    },
    CliResult,
};
use aptos_config::config::{Peer, PeerRole};
use aptos_crypto::{ed25519, x25519, PrivateKey, Uniform, ValidCryptoMaterial};
use aptos_types::account_address::{from_identity_public_key, AccountAddress};
use clap::{Parser, Subcommand};
use rand::SeedableRng;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub const PUBLIC_KEY_EXTENSION: &str = "pub";

/// CLI tool for generating, inspecting, and interacting with keys.
#[derive(Debug, Subcommand)]
pub enum KeyTool {
    Generate(GenerateKey),
    ExtractPeer(ExtractPeer),
}

impl KeyTool {
    pub async fn execute(self) -> CliResult {
        match self {
            KeyTool::Generate(tool) => to_common_result(tool.execute()),
            KeyTool::ExtractPeer(tool) => to_common_result(tool.execute()),
        }
    }
}

/// CLI tool for extracting full peer information for an upstream peer
///
/// A `private-key` or `public-key` can be given encoded on the command line, or
/// a `private-key-file` or a `public-key-file` can be given to read from.
/// The `output_file` will be a YAML serialized peer information for use in network config.
#[derive(Debug, Parser)]
pub struct ExtractPeer {
    #[clap(flatten)]
    private_key_input_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    output_file_options: SaveFile,
    #[clap(flatten)]
    encoding_options: EncodingOptions,
}

impl ExtractPeer {
    pub fn execute(self) -> Result<HashMap<AccountAddress, Peer>, Error> {
        // Check output file exists
        self.output_file_options.check_file()?;

        // Load key based on public or private
        let public_key = self
            .private_key_input_options
            .extract_x25519_public_key(self.encoding_options.encoding)?;

        // Build peer info
        // TODO: Take in an address?
        let peer_id = from_identity_public_key(public_key);
        let mut public_keys = HashSet::new();
        public_keys.insert(public_key);

        let peer = Peer::new(Vec::new(), public_keys, PeerRole::Upstream);

        let mut map = HashMap::new();
        map.insert(peer_id, peer);

        // Save to file
        let yaml =
            serde_yaml::to_string(&map).map_err(|err| Error::UnexpectedError(err.to_string()))?;
        self.output_file_options
            .save_to_file("Extracted peer", yaml.as_bytes())?;
        Ok(map)
    }
}

/// Generates a `x25519` or `ed25519` key.
///
/// This can be used for generating an identity.  Two files will be created
/// `output_file` and `output_file.pub`.  `output_file` will contain the private
/// key encoded with the `encoding` and `output_file.pub` will contain the public
/// key encoded with the `encoding`.
#[derive(Debug, Parser)]
pub struct GenerateKey {
    /// Key type: `x25519` or `ed25519`
    #[clap(long, default_value = "ed25519")]
    key_type: KeyType,
    #[clap(flatten)]
    save_params: SaveKey,
}

impl GenerateKey {
    pub fn execute(self) -> Result<HashMap<&'static str, PathBuf>, Error> {
        self.save_params.check_key_file()?;

        // Generate a ed25519 key
        let ed25519_key = Self::generate_ed25519_in_memory();

        // Convert it to the appropriate type and save it
        match self.key_type {
            KeyType::X25519 => {
                let private_key =
                    x25519::PrivateKey::from_ed25519_private_bytes(&ed25519_key.to_bytes())
                        .map_err(|err| Error::UnexpectedError(err.to_string()))?;
                self.save_params.save_key(&private_key, "x22519")
            }
            KeyType::Ed25519 => self.save_params.save_key(&ed25519_key, "ed22519"),
        }
    }

    /// A test friendly typed key generation for x25519 keys.
    pub fn generate_x25519(
        encoding: EncodingType,
        key_file: &Path,
    ) -> Result<(x25519::PrivateKey, x25519::PublicKey), Error> {
        let args = format!(
            "generate --key-type {key_type:?} --output-file {key_file} --encoding {encoding:?} --assume-yes",
            key_type = KeyType::X25519,
            key_file = key_file.to_str().unwrap(),
            encoding = encoding,
        );
        let command = GenerateKey::parse_from(args.split_whitespace());
        command.execute()?;
        Ok((
            encoding.load_key(key_file)?,
            encoding.load_key(&append_file_extension(key_file, PUBLIC_KEY_EXTENSION)?)?,
        ))
    }

    /// A test friendly typed key generation for e25519 keys.
    pub fn generate_ed25519(
        encoding: EncodingType,
        key_file: &Path,
    ) -> Result<(ed25519::Ed25519PrivateKey, ed25519::Ed25519PublicKey), Error> {
        let args = format!(
            "generate --key-type {key_type:?} --output-file {key_file} --encoding {encoding:?} --assume-yes",
            key_type = KeyType::Ed25519,
            key_file = key_file.to_str().unwrap(),
            encoding = encoding,
        );
        let command = GenerateKey::parse_from(args.split_whitespace());
        command.execute()?;
        Ok((
            encoding.load_key(key_file)?,
            encoding.load_key(&append_file_extension(key_file, PUBLIC_KEY_EXTENSION)?)?,
        ))
    }

    /// Generates an `Ed25519PrivateKey` without saving it to disk
    pub fn generate_ed25519_in_memory() -> ed25519::Ed25519PrivateKey {
        let mut rng = rand::rngs::StdRng::from_entropy();
        ed25519::Ed25519PrivateKey::generate(&mut rng)
    }

    pub fn generate_x25519_in_memory() -> Result<x25519::PrivateKey, Error> {
        let key = Self::generate_ed25519_in_memory();
        x25519::PrivateKey::from_ed25519_private_bytes(&key.to_bytes()).map_err(|err| {
            Error::UnexpectedError(format!("Failed to convert ed25519 to x25519 {:?}", err))
        })
    }
}

#[derive(Debug, Parser)]
pub struct SaveKey {
    #[clap(flatten)]
    file_options: SaveFile,
    #[clap(flatten)]
    encoding_options: EncodingOptions,
}

impl SaveKey {
    /// Public key file name
    fn public_key_file(&self) -> Result<PathBuf, Error> {
        append_file_extension(
            self.file_options.output_file.as_path(),
            PUBLIC_KEY_EXTENSION,
        )
    }

    /// Check if the key file exists already
    pub fn check_key_file(&self) -> Result<(), Error> {
        // Check if file already exists
        self.file_options.check_file()?;
        check_if_file_exists(
            &self.public_key_file()?,
            self.file_options.prompt_options.assume_yes,
        )
    }

    /// Saves a key to a file encoded in a string
    pub fn save_key<Key: PrivateKey + ValidCryptoMaterial>(
        &self,
        key: &Key,
        key_name: &'static str,
    ) -> Result<HashMap<&'static str, PathBuf>, Error> {
        let encoded_private_key = self.encoding_options.encoding.encode_key(key, key_name)?;
        let encoded_public_key = self
            .encoding_options
            .encoding
            .encode_key(&key.public_key(), key_name)?;

        // Write private and public keys to files
        let public_key_file = self.public_key_file()?;
        self.file_options
            .save_to_file(key_name, &encoded_private_key)?;
        write_to_file(&public_key_file, key_name, &encoded_public_key)?;

        let mut map = HashMap::new();
        map.insert("PrivateKey Path", self.file_options.output_file.clone());
        map.insert("PublicKey Path", public_key_file);
        Ok(map)
    }
}
