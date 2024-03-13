// Copyright © Aptos Foundation

use crate::{
    move_any::{Any as MoveAny, AsMoveAny},
    move_fixed_point::FixedPoint64MoveStruct,
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig,
};
use anyhow::anyhow;
use fixed::types::U64F64;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct ConfigOff {}

impl AsMoveAny for ConfigOff {
    const MOVE_TYPE_NAME: &'static str = "0x1::randomness_config::ConfigOff";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct ConfigV1 {
    pub secrecy_threshold: FixedPoint64MoveStruct,
    pub reconstruction_threshold: FixedPoint64MoveStruct,
}

impl Default for ConfigV1 {
    fn default() -> Self {
        Self {
            secrecy_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(1) / U64F64::from_num(2),
            ),
            reconstruction_threshold: FixedPoint64MoveStruct::from_u64f64(
                U64F64::from_num(2) / U64F64::from_num(3),
            ),
        }
    }
}

impl AsMoveAny for ConfigV1 {
    const MOVE_TYPE_NAME: &'static str = "0x1::randomness_config::ConfigV1";
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RandomnessConfigMoveStruct {
    variant: MoveAny,
}

#[derive(Clone, Debug)]
pub enum OnChainRandomnessConfig {
    Off,
    V1(ConfigV1),
}

impl TryFrom<RandomnessConfigMoveStruct> for OnChainRandomnessConfig {
    type Error = anyhow::Error;

    fn try_from(value: RandomnessConfigMoveStruct) -> Result<Self, Self::Error> {
        let RandomnessConfigMoveStruct { variant } = value;
        let variant_type_name = variant.type_name.as_str();
        match variant_type_name {
            ConfigOff::MOVE_TYPE_NAME => Ok(OnChainRandomnessConfig::Off),
            ConfigV1::MOVE_TYPE_NAME => {
                let v1 = MoveAny::unpack(ConfigV1::MOVE_TYPE_NAME, variant)
                    .map_err(|e| anyhow!("unpack as v1 failed: {e}"))?;
                Ok(OnChainRandomnessConfig::V1(v1))
            },
            _ => Err(anyhow!("unknown variant type")),
        }
    }
}

impl From<OnChainRandomnessConfig> for RandomnessConfigMoveStruct {
    fn from(value: OnChainRandomnessConfig) -> Self {
        let variant = match value {
            OnChainRandomnessConfig::Off => MoveAny::pack(ConfigOff::MOVE_TYPE_NAME, ConfigOff {}),
            OnChainRandomnessConfig::V1(v1) => MoveAny::pack(ConfigV1::MOVE_TYPE_NAME, v1),
        };
        RandomnessConfigMoveStruct { variant }
    }
}

impl OnChainRandomnessConfig {
    pub fn default_enabled() -> Self {
        OnChainRandomnessConfig::V1(ConfigV1::default())
    }

    pub fn default_disabled() -> Self {
        OnChainRandomnessConfig::Off
    }

    pub fn default_if_missing() -> Self {
        OnChainRandomnessConfig::Off
    }

    pub fn default_for_genesis() -> Self {
        OnChainRandomnessConfig::Off //TODO: change to `V1` after randomness is ready.
    }

    pub fn randomness_enabled(&self) -> bool {
        match self {
            OnChainRandomnessConfig::Off => false,
            OnChainRandomnessConfig::V1(_) => true,
        }
    }

    pub fn secrecy_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(v1) => Some(v1.secrecy_threshold.as_u64f64()),
        }
    }

    pub fn reconstruct_threshold(&self) -> Option<U64F64> {
        match self {
            OnChainRandomnessConfig::Off => None,
            OnChainRandomnessConfig::V1(v1) => Some(v1.reconstruction_threshold.as_u64f64()),
        }
    }
}

impl OnChainConfig for RandomnessConfigMoveStruct {
    const MODULE_IDENTIFIER: &'static str = "randomness_config";
    const TYPE_IDENTIFIER: &'static str = "RandomnessConfig";
}

impl AsMoveValue for RandomnessConfigMoveStruct {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.variant.as_move_value()]))
    }
}
