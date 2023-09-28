// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

import { sha3_256 as sha3Hash } from "@noble/hashes/sha3";
import { AccountAddress, Hex } from "../core";
import { HexInput } from "../types";
import { MultiEd25519PublicKey } from "./multi_ed25519";
import { PublicKey } from "./asymmetric_crypto";
import { Ed25519PublicKey } from "./ed25519";

/**
 * Each account stores an authentication key. Authentication key enables account owners to rotate
 * their private key(s) associated with the account without changing the address that hosts their account.
 * @see {@link https://aptos.dev/concepts/accounts | Account Basics}
 *
 * Note: AuthenticationKey only supports Ed25519 and MultiEd25519 public keys for now.
 *
 * Account addresses can be derived from AuthenticationKey
 */
export class AuthenticationKey {
  // Length of AuthenticationKey in bytes(Uint8Array)
  static readonly LENGTH: number = 32;

  // Scheme identifier for MultiEd25519 signatures used to derive authentication keys for MultiEd25519 public keys
  static readonly MULTI_ED25519_SCHEME: number = 1;

  // Scheme identifier for Ed25519 signatures used to derive authentication key for MultiEd25519 public key
  static readonly ED25519_SCHEME: number = 0;

  // Scheme identifier used when hashing an account's address together with a seed to derive the address (not the
  // authentication key) of a resource account.
  static readonly DERIVE_RESOURCE_ACCOUNT_SCHEME: number = 255;

  // Actual data of AuthenticationKey, in Hex format
  public readonly data: Hex;

  constructor(args: { data: HexInput }) {
    const { data } = args;
    const hex = Hex.fromHexInput({ hexInput: data });
    if (hex.toUint8Array().length !== AuthenticationKey.LENGTH) {
      throw new Error(`Authentication Key length should be ${AuthenticationKey.LENGTH}`);
    }
    this.data = hex;
  }

  toString(): string {
    return this.data.toString();
  }

  toUint8Array(): Uint8Array {
    return this.data.toUint8Array();
  }

  /**
   * Converts a PublicKey(s) to AuthenticationKey
   *
   * @param publicKey
   * @returns AuthenticationKey
   */
  static fromPublicKey(args: { publicKey: PublicKey }): AuthenticationKey {
    const { publicKey } = args;

    let scheme: number;
    if (publicKey instanceof Ed25519PublicKey) {
      scheme = AuthenticationKey.ED25519_SCHEME;
    } else if (publicKey instanceof MultiEd25519PublicKey) {
      scheme = AuthenticationKey.MULTI_ED25519_SCHEME;
    } else {
      throw new Error("Unsupported authentication key scheme");
    }

    const pubKeyBytes = publicKey.toUint8Array();

    const bytes = new Uint8Array(pubKeyBytes.length + 1);
    bytes.set(pubKeyBytes);
    bytes.set([scheme], pubKeyBytes.length);

    const hash = sha3Hash.create();
    hash.update(bytes);

    return new AuthenticationKey({ data: hash.digest() });
  }

  /**
   * Derives an account address from AuthenticationKey. Since current AccountAddress is 32 bytes,
   * AuthenticationKey bytes are directly translated to AccountAddress.
   *
   * @returns AccountAddress
   */
  derivedAddress(): AccountAddress {
    return new AccountAddress({ data: this.data.toUint8Array() });
  }
}
