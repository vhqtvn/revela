// Copyright © Aptos Foundation

#[cfg(test)]
use crate::move_any::Any as MoveAny;
use crate::{move_any::AsMoveAny, move_utils::as_move_value::AsMoveValue, zkid::Claims};
use anyhow::{anyhow, bail, ensure, Result};
use aptos_crypto::poseidon_bn254;
use base64::URL_SAFE_NO_PAD;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::str::FromStr;

pub const RSA_MODULUS_BYTES: usize = 256;

/// Move type `0x1::jwks::RSA_JWK` in rust.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RSA_JWK {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub e: String,
    pub n: String,
}

impl RSA_JWK {
    #[cfg(test)]
    pub fn new_for_testing(kid: &str, kty: &str, alg: &str, e: &str, n: &str) -> Self {
        Self {
            kid: kid.to_string(),
            kty: kty.to_string(),
            alg: alg.to_string(),
            e: e.to_string(),
            n: n.to_string(),
        }
    }

    // TODO(zkid): Move this to aptos-crypto so other services can use this
    pub fn to_poseidon_scalar(&self) -> Result<ark_bn254::Fr> {
        let mut modulus = base64::decode_config(&self.n, URL_SAFE_NO_PAD)?;
        // The circuit only supports RSA256
        if modulus.len() != RSA_MODULUS_BYTES {
            bail!("Wrong modulus size, must be {} bytes", RSA_MODULUS_BYTES);
        }
        modulus.reverse(); // This is done to match the circuit, which requires the modulus in a verify specific format due to how RSA verification is implemented
                           // TODO(zkid): finalize the jwk hashing scheme.
        let mut scalars = modulus
            .chunks(24) // Pack 3 64 bit limbs per scalar, so chunk into 24 bytes per scalar
            .map(|chunk| {
                poseidon_bn254::pack_bytes_to_one_scalar(chunk).expect("chunk converts to scalar")
            })
            .collect::<Vec<ark_bn254::Fr>>();
        scalars.push(ark_bn254::Fr::from(RSA_MODULUS_BYTES as i32));
        poseidon_bn254::hash_scalars(scalars)
    }

    pub fn verify_signature(&self, jwt_token: &str) -> Result<TokenData<Claims>> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        let key = &DecodingKey::from_rsa_components(&self.n, &self.e)?;
        let claims = jsonwebtoken::decode::<Claims>(jwt_token, key, &validation)?;
        Ok(claims)
    }
}

impl AsMoveAny for RSA_JWK {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwks::RSA_JWK";
}

impl TryFrom<&serde_json::Value> for RSA_JWK {
    type Error = anyhow::Error;

    fn try_from(json_value: &serde_json::Value) -> Result<Self, Self::Error> {
        let kty = json_value
            .get("kty")
            .ok_or_else(|| anyhow!("Field `kty` not found"))?
            .as_str()
            .ok_or_else(|| anyhow!("Field `kty` is not a string"))?
            .to_string();

        ensure!(
            kty.as_str() == "RSA",
            "json to rsa jwk conversion failed with incorrect kty"
        );

        let ret = Self {
            kty,
            kid: json_value
                .get("kid")
                .ok_or_else(|| anyhow!("Field `kid` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `kid` is not a string"))?
                .to_string(),
            alg: json_value
                .get("alg")
                .ok_or_else(|| anyhow!("Field `alg` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `alg` is not a string"))?
                .to_string(),
            e: json_value
                .get("e")
                .ok_or_else(|| anyhow!("Field `e` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `e` is not a string"))?
                .to_string(),
            n: json_value
                .get("n")
                .ok_or_else(|| anyhow!("Field `n` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `n` is not a string"))?
                .to_string(),
        };

        Ok(ret)
    }
}

impl AsMoveValue for RSA_JWK {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.kid.as_move_value(),
            self.kty.as_move_value(),
            self.alg.as_move_value(),
            self.e.as_move_value(),
            self.n.as_move_value(),
        ]))
    }
}

#[test]
fn test_rsa_jwk_from_json() {
    // Valid JWK JSON should be accepted.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    let actual = RSA_JWK::try_from(&json);
    let expected = RSA_JWK::new_for_testing("kid1", "RSA", "RS256", "AQAB", "13131");
    assert_eq!(expected, actual.unwrap());

    // JWK JSON without `kid` should be rejected.
    let json_str = r#"{"alg": "RS256", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `kid` type should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": {}, "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `alg` should be rejected.
    let json_str = r#"{"kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `alg` type should be rejected.
    let json_str =
        r#"{"alg": 0, "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `kty` should be rejected.
    let json_str = r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `kty` value should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSB", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `e` should be rejected.
    let json_str = r#"{"alg": "RS256", "kid": "kid1", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `e` type should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": 65537, "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `n` should be rejected.
    let json_str = r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `n` type should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": false}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());
}

#[test]
fn test_rsa_jwk_as_move_value() {
    let rsa_jwk = RSA_JWK::new_for_testing("kid1", "RSA", "RS256", "AQAB", "13131");
    let move_value = rsa_jwk.as_move_value();
    assert_eq!(
        vec![
            4, 107, 105, 100, 49, 3, 82, 83, 65, 5, 82, 83, 50, 53, 54, 4, 65, 81, 65, 66, 5, 49,
            51, 49, 51, 49
        ],
        move_value.simple_serialize().unwrap()
    );
}

#[test]
fn test_rsa_jwk_as_move_any() {
    let rsa_jwk = RSA_JWK::new_for_testing("kid1", "RSA", "RS256", "AQAB", "1313131313131");
    let actual = rsa_jwk.as_move_any();
    let expected = MoveAny {
        type_name: "0x1::jwks::RSA_JWK".to_string(),
        data: bcs::to_bytes(&rsa_jwk).unwrap(),
    };
    assert_eq!(expected, actual);
}
