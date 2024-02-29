// Copyright © Aptos Foundation

use crate::keyless::{
    bn254_circom::get_public_inputs_hash,
    circuit_testcases::*,
    test_utils::{get_sample_groth16_sig_and_pk, get_sample_openid_sig_and_pk},
    Configuration, ZkpOrOpenIdSig, DEVNET_VERIFICATION_KEY,
};
use std::ops::{AddAssign, Deref};

// TODO(keyless): Add instructions on how to produce this test case.
#[test]
fn test_keyless_groth16_proof_verification() {
    let config = Configuration::new_for_devnet();

    let (zk_sig, zk_pk) = get_sample_groth16_sig_and_pk();
    let proof = match &zk_sig.sig {
        ZkpOrOpenIdSig::Groth16Zkp(proof) => proof.clone(),
        ZkpOrOpenIdSig::OpenIdSig(_) => panic!("Internal inconsistency"),
    };

    let public_inputs_hash = get_public_inputs_hash(&zk_sig, &zk_pk, &SAMPLE_JWK, &config).unwrap();

    println!(
        "Keyless Groth16 test public inputs hash: {}",
        public_inputs_hash
    );

    proof
        .verify_proof(public_inputs_hash, DEVNET_VERIFICATION_KEY.deref())
        .unwrap();
}

#[test]
fn test_keyless_oidc_sig_verifies() {
    // Verification should succeed
    let config = Configuration::new_for_testing();
    let (sig, pk) = get_sample_openid_sig_and_pk();

    let oidc_sig = match &sig.sig {
        ZkpOrOpenIdSig::Groth16Zkp(_) => panic!("Internal inconsistency"),
        ZkpOrOpenIdSig::OpenIdSig(oidc_sig) => oidc_sig.clone(),
    };

    oidc_sig
        .verify_jwt_claims(sig.exp_date_secs, &sig.ephemeral_pubkey, &pk, &config)
        .unwrap();

    oidc_sig
        .verify_jwt_signature(&SAMPLE_JWK, &sig.jwt_header_json)
        .unwrap();

    // Maul the pepper; verification should fail
    let mut bad_oidc_sig = oidc_sig.clone();
    bad_oidc_sig.pepper.0[0].add_assign(1);
    assert_ne!(bad_oidc_sig.pepper, oidc_sig.pepper);

    let e = bad_oidc_sig
        .verify_jwt_claims(sig.exp_date_secs, &sig.ephemeral_pubkey, &pk, &config)
        .unwrap_err();
    assert!(e.to_string().contains("IDC verification failed"));

    // Expiration date is past the expiration horizon; verification should fail
    let bad_oidc_sig = oidc_sig.clone();
    let e = bad_oidc_sig
        .verify_jwt_claims(
            SAMPLE_JWT_PARSED.oidc_claims.iat + config.max_exp_horizon_secs,
            &sig.ephemeral_pubkey,
            &pk,
            &config,
        )
        .unwrap_err();
    assert!(e.to_string().contains("expiration date is too far"));

    // `sub` field does not match IDC; verification should fail
    let mut bad_oidc_sig = oidc_sig.clone();
    let mut jwt = SAMPLE_JWT_PARSED.clone();
    jwt.oidc_claims.sub = format!("{}+1", SAMPLE_JWT_PARSED.oidc_claims.sub);
    bad_oidc_sig.jwt_payload_json = serde_json::to_string(&jwt).unwrap();

    let e = bad_oidc_sig
        .verify_jwt_claims(sig.exp_date_secs, &sig.ephemeral_pubkey, &pk, &config)
        .unwrap_err();
    assert!(e.to_string().contains("IDC verification failed"));

    // `nonce` field is wrong; verification should fail
    let mut bad_oidc_sig = oidc_sig.clone();
    let mut jwt = SAMPLE_JWT_PARSED.clone();
    jwt.oidc_claims.nonce = "bad nonce".to_string();
    bad_oidc_sig.jwt_payload_json = serde_json::to_string(&jwt).unwrap();

    let e = bad_oidc_sig
        .verify_jwt_claims(sig.exp_date_secs, &sig.ephemeral_pubkey, &pk, &config)
        .unwrap_err();
    assert!(e.to_string().contains("'nonce' claim"));

    // `iss` field is wrong; verification should fail
    let mut bad_oidc_sig = oidc_sig.clone();
    let mut jwt = SAMPLE_JWT_PARSED.clone();
    jwt.oidc_claims.iss = "bad iss".to_string();
    bad_oidc_sig.jwt_payload_json = serde_json::to_string(&jwt).unwrap();

    let e = bad_oidc_sig
        .verify_jwt_claims(sig.exp_date_secs, &sig.ephemeral_pubkey, &pk, &config)
        .unwrap_err();
    assert!(e.to_string().contains("'iss' claim "));
}
