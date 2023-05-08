// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]

use super::token_utils::{NAME_LENGTH, URI_LENGTH};
use crate::{
    models::{move_resources::MoveResource, v2_objects::CurrentObjectPK},
    util::{
        deserialize_token_object_property_map_from_bcs_hexstring, standardize_address, truncate_str,
    },
};
use anyhow::{Context, Result};
use aptos_api_types::{deserialize_from_string, Event, WriteResource};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Formatter},
};

/// Tracks all token related data in a hashmap for quick access (keyed on address of the object core)
pub type TokenV2AggregatedDataMapping = HashMap<CurrentObjectPK, TokenV2AggregatedData>;
/// Tracks all token related data in a hashmap for quick access (keyed on address of the object core)
pub type TokenV2Burned = HashSet<CurrentObjectPK>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TokenV2AggregatedData {
    pub aptos_collection: Option<AptosCollection>,
    pub fixed_supply: Option<FixedSupply>,
    pub object: ObjectCore,
    pub unlimited_supply: Option<UnlimitedSupply>,
    pub property_map: Option<PropertyMap>,
}

/// Tracks which token standard a token / collection is built upon
#[derive(Serialize)]
pub enum TokenStandard {
    V1,
    V2,
}

impl fmt::Display for TokenStandard {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let res = match self {
            TokenStandard::V1 => "v1",
            TokenStandard::V2 => "v2",
        };
        write!(f, "{}", res)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ObjectCore {
    pub allow_ungated_transfer: bool,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub guid_creation_num: BigDecimal,
    pub owner: String,
}

impl ObjectCore {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        if let V2TokenResource::ObjectCore(inner) = V2TokenResource::from_resource(
            &type_str,
            &serde_json::to_value(&write_resource.data.data).unwrap(),
            txn_version,
        )? {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Collection {
    pub creator: String,
    pub description: String,
    // These are set to private because we should never get name or uri directly
    name: String,
    uri: String,
}

impl Collection {
    pub fn get_uri_trunc(&self) -> String {
        truncate_str(&self.uri, URI_LENGTH)
    }

    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, NAME_LENGTH)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AptosCollection {
    pub mutable_description: bool,
    pub mutable_uri: bool,
}

impl AptosCollection {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::AptosCollection(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    pub collection: ResourceReference,
    pub description: String,
    // These are set to private because we should never get name or uri directly
    name: String,
    uri: String,
}

impl Token {
    pub fn get_uri_trunc(&self) -> String {
        truncate_str(&self.uri, URI_LENGTH)
    }

    pub fn get_name_trunc(&self) -> String {
        truncate_str(&self.name, NAME_LENGTH)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceReference {
    pub inner: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FixedSupply {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub current_supply: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub max_supply: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_minted: BigDecimal,
}

impl FixedSupply {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::FixedSupply(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnlimitedSupply {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub current_supply: BigDecimal,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub total_minted: BigDecimal,
}

impl UnlimitedSupply {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::UnlimitedSupply(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BurnEvent {
    #[serde(deserialize_with = "deserialize_from_string")]
    pub index: BigDecimal,
    token: String,
}

impl BurnEvent {
    pub fn from_event(event: &Event, txn_version: i64) -> anyhow::Result<Option<Self>> {
        let event_type = event.typ.to_string();
        if let Some(V2TokenEvent::BurnEvent(inner)) =
            V2TokenEvent::from_event(event_type.as_str(), &event.data, txn_version).unwrap()
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }

    pub fn get_token_address(&self) -> String {
        standardize_address(&self.token)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PropertyMap {
    #[serde(deserialize_with = "deserialize_token_object_property_map_from_bcs_hexstring")]
    pub inner: serde_json::Value,
}

impl PropertyMap {
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
    ) -> anyhow::Result<Option<Self>> {
        let type_str = format!(
            "{}::{}::{}",
            write_resource.data.typ.address,
            write_resource.data.typ.module,
            write_resource.data.typ.name
        );
        if !V2TokenResource::is_resource_supported(type_str.as_str()) {
            return Ok(None);
        }
        let resource = MoveResource::from_write_resource(
            write_resource,
            0, // Placeholder, this isn't used anyway
            txn_version,
            0, // Placeholder, this isn't used anyway
        );

        if let V2TokenResource::PropertyMap(inner) =
            V2TokenResource::from_resource(&type_str, resource.data.as_ref().unwrap(), txn_version)?
        {
            Ok(Some(inner))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum V2TokenResource {
    AptosCollection(AptosCollection),
    Collection(Collection),
    FixedSupply(FixedSupply),
    ObjectCore(ObjectCore),
    UnlimitedSupply(UnlimitedSupply),
    Token(Token),
    PropertyMap(PropertyMap),
}

impl V2TokenResource {
    pub fn is_resource_supported(data_type: &str) -> bool {
        matches!(
            data_type,
            "0x1::object::ObjectCore"
                | "0x4::collection::Collection"
                | "0x4::collection::FixedSupply"
                | "0x4::collection::UnlimitedSupply"
                | "0x4::aptos_token::AptosCollection"
                | "0x4::token::Token"
                | "0x4::property_map::PropertyMap"
        )
    }

    pub fn from_resource(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Self> {
        match data_type {
            "0x1::object::ObjectCore" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::ObjectCore(inner)))
            },
            "0x4::collection::Collection" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::Collection(inner)))
            },
            "0x4::collection::FixedSupply" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::FixedSupply(inner)))
            },
            "0x4::collection::UnlimitedSupply" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::UnlimitedSupply(inner)))
            },
            "0x4::aptos_token::AptosCollection" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::AptosCollection(inner)))
            },
            "0x4::token::Token" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::Token(inner)))
            },
            "0x4::property_map::PropertyMap" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::PropertyMap(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))?
        .context(format!(
            "Resource unsupported! Call is_resource_supported first. version {} type {}",
            txn_version, data_type
        ))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum V2TokenEvent {
    BurnEvent(BurnEvent),
}

impl V2TokenEvent {
    pub fn from_event(
        data_type: &str,
        data: &serde_json::Value,
        txn_version: i64,
    ) -> Result<Option<V2TokenEvent>> {
        match data_type {
            "0x4::collection::BurnEvent" => {
                serde_json::from_value(data.clone()).map(|inner| Some(Self::BurnEvent(inner)))
            },
            _ => Ok(None),
        }
        .context(format!(
            "version {} failed! failed to parse type {}, data {:?}",
            txn_version, data_type, data
        ))
    }
}
