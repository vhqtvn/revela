// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::authenticator::EphemeralPublicKey;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub mod jwt;
pub mod vuf;

/// Custom serialization function to convert Vec<u8> into a hex string.
fn serialize_bytes_to_hex<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex_string = hex::encode(bytes);
    serializer.serialize_str(&hex_string)
}

/// Custom deserialization function to convert a hex string back into Vec<u8>.
fn deserialize_bytes_from_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    hex::decode(s).map_err(D::Error::custom)
}

/// Custom serialization function to convert `EphemeralPublicKey` into a hex string.
fn serialize_epk_to_hex<S>(epk: &EphemeralPublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let epk_bytes = epk.to_bytes();
    serialize_bytes_to_hex(&epk_bytes, serializer)
}

fn deserialize_epk_from_hex<'de, D>(deserializer: D) -> Result<EphemeralPublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = deserialize_bytes_from_hex(deserializer)?;
    let pk = EphemeralPublicKey::try_from(bytes.as_slice()).map_err(D::Error::custom)?;
    Ok(pk)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BadPepperRequestError {
    pub message: String,
}

/// A pepper scheme where:
/// - The pepper input contains `JWT, epk, blinder, expiry_time, uid_key`, wrapped in type `PepperRequest`.
/// - The pepper output is the `BLS12381_G1_BLS` VUF output of the input, wrapped in type `PepperResponse`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperRequest {
    #[serde(rename = "jwt_b64")]
    pub jwt: String,
    #[serde(
        serialize_with = "serialize_epk_to_hex",
        deserialize_with = "deserialize_epk_from_hex"
    )]
    pub epk: EphemeralPublicKey,
    pub exp_date_secs: u64,
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub epk_blinder: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid_key: Option<String>,
}

/// The response to `PepperRequest`, which contains either the pepper or a processing error.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperResponse {
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub signature: Vec<u8>, // unique BLS signature
}

/// The response to `/v0/vuf-pub-key`.
/// NOTE that in pepper v0, VUF is fixed to be `BLS12381_G1_BLS`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperV0VufPubKey {
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub public_key: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PepperInput {
    pub iss: String,
    pub aud: String,
    pub uid_val: String,
    pub uid_key: String,
}
