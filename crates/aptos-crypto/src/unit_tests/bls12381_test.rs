// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bls12381,
    bls12381::{
        bls12381_keys::{PrivateKey, PublicKey},
        ProofOfPossession,
    },
    test_utils::{random_subset, KeyPair, TestAptosCrypto},
    Signature, SigningKey, Uniform,
};
use rand::{distributions::Alphanumeric, Rng};
use rand_core::OsRng;
use std::iter::zip;

/// Tests that an individual signature share computed correctly on a message m passes verification on m.
/// Tests that a signature share computed on a different message m' fails verification on m.
/// Tests that a signature share fails verification under the wrong public key.
#[test]
fn bls12381_sigshare_verify() {
    let mut rng = OsRng;

    let message = b"Hello world";
    let message_wrong = b"Wello Horld";

    let key_pair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
    let key_pair_wrong = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    let signature = key_pair.private_key.sign_arbitrary_message(message);
    let signature_wrong = key_pair_wrong.private_key.sign_arbitrary_message(message);

    // sig on message under key_pair should verify
    assert!(signature
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .is_ok());

    // sig_wrong on message under key_pair_wrong should verify
    assert!(signature_wrong
        .verify_arbitrary_msg(message, &key_pair_wrong.public_key)
        .is_ok());

    // sig on message under keypair should NOT verify under keypair_wrong
    assert!(signature
        .verify_arbitrary_msg(message, &key_pair_wrong.public_key)
        .is_err());

    // sig on message under keypair should NOT verify on message_wrong under key_pair
    assert!(signature
        .verify_arbitrary_msg(message_wrong, &key_pair.public_key)
        .is_err());

    // sig on message under keypair_wrong should NOT verify under key_pair
    assert!(signature_wrong
        .verify_arbitrary_msg(message, &key_pair.public_key)
        .is_err());
}

/// Tests that a PoP for PK 1 verifies under PK 1.
/// Tests that a PoP for PK 1 does NOT verify under PK 2.
#[test]
fn bls12381_pop_verify() {
    let mut rng = OsRng;

    let keypair1 = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    let keypair2 = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    // Valid PoP for SK 1
    let pop1 = ProofOfPossession::create_with_pubkey(&keypair1.private_key, &keypair1.public_key);
    // Valid PoP for SK 2
    let pop2 = ProofOfPossession::create(&keypair2.private_key);
    // Invalid PoP for SK 2
    let pop_bad =
        ProofOfPossession::create_with_pubkey(&keypair1.private_key, &keypair2.public_key);

    // PoP for SK i should verify for PK i
    assert!(pop1.verify(&keypair1.public_key).is_ok());
    assert!(pop2.verify(&keypair2.public_key).is_ok());

    // PoP for SK 1 should not verify for PK 2
    assert!(pop1.verify(&keypair2.public_key).is_err());
    // Pop for SK 2 should not verify for PK 1
    assert!(pop2.verify(&keypair1.public_key).is_err());
    // Invalid PoP for SK 2 should not verify
    assert!(pop_bad.verify(&keypair2.public_key).is_err());
}

/// Generates `num_signers` BLS key-pairs.
fn bls12381_keygen(num_signers: usize, mut rng: &mut OsRng) -> Vec<KeyPair<PrivateKey, PublicKey>> {
    let mut key_pairs = vec![];
    for _ in 0..num_signers {
        key_pairs.push(KeyPair::<PrivateKey, PublicKey>::generate(&mut rng));
    }
    key_pairs
}

/// Returns a 256-character unique string that can be signed by the BLS API.
fn random_message_for_signing(rng: &mut OsRng) -> TestAptosCrypto {
    TestAptosCrypto(
        rng.sample_iter(&Alphanumeric)
            .take(256)
            .map(char::from)
            .collect::<String>(),
    )
}

/// Tests that a multisignature on a message m aggregated from n/2 out of n signers verifies
/// correctly on m but fails verification on a different m'.
#[test]
fn bls12381_multisig_should_verify() {
    let mut rng = OsRng;

    let message = random_message_for_signing(&mut rng);
    let message_wrong = random_message_for_signing(&mut rng);

    let num_signers = 1000;
    let key_pairs = bls12381_keygen(num_signers, &mut rng);

    let mut signatures = vec![];
    let mut pubkeys: Vec<&PublicKey> = vec![];

    let good_step = 2;
    for keys in key_pairs.iter().step_by(good_step) {
        let signature = keys.private_key.sign(&message);
        signatures.push(signature);
        pubkeys.push(&keys.public_key);
    }

    let multisig = bls12381::Signature::aggregate(signatures).unwrap();
    let aggpk = PublicKey::aggregate(pubkeys).unwrap();

    // multisig should verify on the correct message under the correct aggregate PK
    assert!(multisig.verify(&message, &aggpk).is_ok());

    // multisig should not verify on an incorrect message under the correct aggregate PK
    assert!(multisig.verify(&message_wrong, &aggpk).is_err());
}

/// Tests that a multisig incorrectly aggregated from signature shares on different messages does
/// NOT verify.
#[test]
fn bls12381_multisig_wrong_messages_aggregated() {
    let mut rng = OsRng;

    let message = random_message_for_signing(&mut rng);
    let message_wrong = random_message_for_signing(&mut rng);

    let num_signers = 500;
    let key_pairs = bls12381_keygen(num_signers, &mut rng);
    assert_eq!(key_pairs.len(), num_signers);

    let mut signatures = vec![];
    let mut pubkeys: Vec<&PublicKey> = vec![];

    for (i, key_pair) in key_pairs.iter().enumerate() {
        let signature = if i % 2 == 0 {
            key_pair.private_key.sign(&message)
        } else {
            key_pair.private_key.sign(&message_wrong)
        };
        signatures.push(signature);
        pubkeys.push(&key_pair.public_key);
    }

    let multisig = bls12381::Signature::aggregate(signatures).unwrap();
    let aggpk = PublicKey::aggregate(pubkeys).unwrap();

    // multisig should NOT verify on any of the messages, because it is actually not a multisig:
    // i.e., it is not an aggregate signature on a single message
    assert!(multisig.verify(&message, &aggpk).is_err());
    assert!(multisig.verify(&message_wrong, &aggpk).is_err());
}

/// Returns two different sets of signer IDs (i.e., numbers in 0..num_signers)
pub fn random_different_signer_sets(
    rng: &mut OsRng,
    num_signers: usize,
    subset_size: usize,
) -> (Vec<usize>, Vec<usize>) {
    let signers1 = random_subset(rng, num_signers, subset_size);
    let mut signers2 = random_subset(rng, num_signers, subset_size);

    while signers1 == signers2 {
        signers2 = random_subset(rng, num_signers, subset_size);
    }

    (signers1, signers2)
}

/// Tests that a multisig aggregated from a set of signers A does not verify under a public key
/// aggregated from a different set B of signers.
#[test]
fn bls12381_multisig_wrong_pks_aggregated() {
    let mut rng = OsRng;

    let message1 = random_message_for_signing(&mut rng);
    let message2 = random_message_for_signing(&mut rng);

    let num_signers = 1000;
    let key_pairs = bls12381_keygen(num_signers, &mut rng);
    assert_eq!(key_pairs.len(), num_signers);

    let (signers1, signers2) = random_different_signer_sets(&mut rng, num_signers, num_signers / 2);

    let mut signatures1 = vec![];
    let mut signatures2 = vec![];
    let mut pubkeys1 = vec![];
    let mut pubkeys2 = vec![];

    for (i1, i2) in zip(signers1, signers2) {
        signatures1.push(key_pairs[i1].private_key.sign(&message1));
        signatures2.push(key_pairs[i2].private_key.sign(&message2));

        pubkeys1.push(&key_pairs[i1].public_key);
        pubkeys2.push(&key_pairs[i2].public_key);
    }
    assert_ne!(signatures1.len(), 0);
    assert_ne!(signatures2.len(), 0);

    let multisig1 = bls12381::Signature::aggregate(signatures1).unwrap();
    let multisig2 = bls12381::Signature::aggregate(signatures2).unwrap();
    let aggpk1 = PublicKey::aggregate(pubkeys1).unwrap();
    let aggpk2 = PublicKey::aggregate(pubkeys2).unwrap();

    // first, make sure multisig1 (and multisig2) verify on message1 (and on message2) under aggpk1 (and aggpk2, respectively)
    assert!(multisig1.verify(&message1, &aggpk1).is_ok());
    assert!(multisig2.verify(&message2, &aggpk2).is_ok());

    // second, make sure multisig1 doesn't verify against multisig2's signer set (and viceversa)
    assert!(multisig1.verify(&message1, &aggpk2).is_err());
    assert!(multisig2.verify(&message2, &aggpk1).is_err());

    // ...and try swapping the messages too
    assert!(multisig1.verify(&message2, &aggpk2).is_err());
    assert!(multisig2.verify(&message1, &aggpk1).is_err());
}

/// Tests that a randomly generated multisig does not verify under a randomly generated PK.
#[test]
fn bls12381_random_multisig_dont_verify_with_random_pk() {
    let mut rng = OsRng;

    let message = random_message_for_signing(&mut rng);
    let keypair = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);
    let keypair_junk = KeyPair::<PrivateKey, PublicKey>::generate(&mut rng);

    let signature = keypair.private_key.sign(&message);

    assert!(signature.verify(&message, &keypair.public_key).is_ok());

    assert!(signature
        .verify(&message, &keypair_junk.public_key)
        .is_err());
}
