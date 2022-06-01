import { HexString } from "../../hex_string";
import {
  Deserializer,
  Serializer,
  uint64,
  bytes,
  Seq,
  uint8,
  bool,
  uint128,
  deserializeVector,
  serializeVector,
} from "../bcs";
import { AccountAddress } from "./account_address";
import { TransactionAuthenticator } from "./authenticator";
import { Identifier } from "./identifier";
import { TypeTag } from "./type_tag";

export class RawTransaction {
  /**
   * RawTransactions contain the metadata and payloads that can be submitted to Aptos chain for execution.
   * RawTransactions must be signed before Aptos chain can execute them.
   *
   * @param sender Account address of the sender.
   * @param sequence_number Sequence number of this transaction. This must match the sequence number stored in
   *   the sender's account at the time the transaction executes.
   * @param payload Instructions for the Aptos Blockchain, including publishing a module,
   *   execute a script function or execute a script payload.
   * @param max_gas_amount Maximum total gas to spend for this transaction. The account must have more
   *   than this gas or the transaction will be discarded during validation.
   * @param gas_unit_price Price to be paid per gas unit.
   * @param expiration_timestamp_secs The blockchain timestamp at which the blockchain would discard this transaction.
   * @param chain_id The chain ID of the blockchain that this transaction is intended to be run on.
   */
  constructor(
    public readonly sender: AccountAddress,
    public readonly sequence_number: uint64,
    public readonly payload: TransactionPayload,
    public readonly max_gas_amount: uint64,
    public readonly gas_unit_price: uint64,
    public readonly expiration_timestamp_secs: uint64,
    public readonly chain_id: ChainId,
  ) {}

  serialize(serializer: Serializer): void {
    this.sender.serialize(serializer);
    serializer.serializeU64(this.sequence_number);
    this.payload.serialize(serializer);
    serializer.serializeU64(this.max_gas_amount);
    serializer.serializeU64(this.gas_unit_price);
    serializer.serializeU64(this.expiration_timestamp_secs);
    this.chain_id.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): RawTransaction {
    const sender = AccountAddress.deserialize(deserializer);
    const sequence_number = deserializer.deserializeU64();
    const payload = TransactionPayload.deserialize(deserializer);
    const max_gas_amount = deserializer.deserializeU64();
    const gas_unit_price = deserializer.deserializeU64();
    const expiration_timestamp_secs = deserializer.deserializeU64();
    const chain_id = ChainId.deserialize(deserializer);
    return new RawTransaction(
      sender,
      sequence_number,
      payload,
      max_gas_amount,
      gas_unit_price,
      expiration_timestamp_secs,
      chain_id,
    );
  }
}

export class Script {
  /**
   * Scripts contain the Move bytecodes payload that can be submitted to Aptos chain for execution.
   * @param code Move bytecode
   * @param ty_args Type arguments that bytecode requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args Arugments to bytecode function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   */
  constructor(
    public readonly code: bytes,
    public readonly ty_args: Seq<TypeTag>,
    public readonly args: Seq<TransactionArgument>,
  ) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.code);
    serializeVector<TypeTag>(this.ty_args, serializer);
    serializeVector<TransactionArgument>(this.args, serializer);
  }

  static deserialize(deserializer: Deserializer): Script {
    const code = deserializer.deserializeBytes();
    const ty_args = deserializeVector(deserializer, TypeTag);
    const args = deserializeVector(deserializer, TransactionArgument);
    return new Script(code, ty_args, args);
  }
}

export class ScriptFunction {
  /**
   * Contains the payload to run a function within a module.
   * @param module_name Fullly qualified module name. ModuleId consists of account address and module name.
   * @param function_name The function to run.
   * @param ty_args Type arguments that move function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args Arugments to the move function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   */
  constructor(
    public readonly module_name: ModuleId,
    public readonly function_name: Identifier,
    public readonly ty_args: Seq<TypeTag>,
    public readonly args: Seq<bytes>,
  ) {}

  /**
   *
   * @param module Fully qualified module name in format "AccountAddress::ModuleName" e.g. "0x1::Coin"
   * @param func Function name
   * @param ty_args Type arguments that move function requires.
   *
   * @example
   * A coin transfer function has one type argument "CoinType".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @param args Arugments to the move function.
   *
   * @example
   * A coin transfer function has three arugments "from", "to" and "amount".
   * ```
   * public(script) fun transfer<CoinType>(from: &signer, to: address, amount: u64,)
   * ```
   * @returns
   */
  static natual(module: string, func: string, ty_args: Seq<TypeTag>, args: Seq<bytes>): ScriptFunction {
    return new ScriptFunction(ModuleId.fromStr(module), new Identifier(func), ty_args, args);
  }

  serialize(serializer: Serializer): void {
    this.module_name.serialize(serializer);
    this.function_name.serialize(serializer);
    serializeVector<TypeTag>(this.ty_args, serializer);
    serializeVectorBytes(this.args, serializer);
  }

  static deserialize(deserializer: Deserializer): ScriptFunction {
    const module_name = ModuleId.deserialize(deserializer);
    const function_name = Identifier.deserialize(deserializer);
    const ty_args = deserializeVector(deserializer, TypeTag);
    const args = deserializeVectorBytes(deserializer);
    return new ScriptFunction(module_name, function_name, ty_args, args);
  }
}

export class Module {
  /**
   * Contains the bytecode of a Move module that can be published to the Aptos chain.
   * @param code Move bytecode of a module.
   */
  constructor(public readonly code: bytes) {}

  serialize(serializer: Serializer): void {
    serializer.serializeBytes(this.code);
  }

  static deserialize(deserializer: Deserializer): Module {
    const code = deserializer.deserializeBytes();
    return new Module(code);
  }
}

export class ModuleBundle {
  /**
   * Contains a list of Modules that can be published together.
   * @param codes List of modules.
   */
  constructor(public readonly codes: Seq<Module>) {}

  serialize(serializer: Serializer): void {
    serializeVector<Module>(this.codes, serializer);
  }

  static deserialize(deserializer: Deserializer): ModuleBundle {
    const codes = deserializeVector(deserializer, Module);
    return new ModuleBundle(codes);
  }
}

export class ModuleId {
  /**
   * Full name of a module.
   * @param address The account address.
   * @param name The name of the module under the account at "address".
   */
  constructor(public readonly address: AccountAddress, public readonly name: Identifier) {}

  static fromStr(moduleId: string): ModuleId {
    const parts = moduleId.split("::");
    if (parts.length !== 2) {
      throw new Error("Invalid module id.");
    }
    return new ModuleId(AccountAddress.fromHex(new HexString(parts[0])), new Identifier(parts[1]));
  }

  serialize(serializer: Serializer): void {
    this.address.serialize(serializer);
    this.name.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): ModuleId {
    const address = AccountAddress.deserialize(deserializer);
    const name = Identifier.deserialize(deserializer);
    return new ModuleId(address, name);
  }
}

export class ChangeSet {
  serialize(serializer: Serializer): void {
    throw new Error("Not implemented.");
  }

  static deserialize(deserializer: Deserializer): ChangeSet {
    throw new Error("Not implemented.");
  }
}

export class WriteSet {
  serialize(serializer: Serializer): void {
    throw new Error("Not implmented.");
  }

  static deserialize(deserializer: Deserializer): WriteSet {
    throw new Error("Not implmented.");
  }
}

export class SignedTransaction {
  /**
   * A SignedTransaction consists of a raw transaction and an authenticator. The authenticator
   * contains a client's public key and the signature of the raw transaction.
   *
   * @see {@link https://aptos.dev/guides/creating-a-signed-transaction/ | Creating a Signed Transaction}
   *
   * @param raw_txn
   * @param authenticator Contains a client's public key and the signature of the raw transaction.
   *   Authenticator has 3 flavors: single signature, multi-signature and multi-agent.
   *   @see authenticator.ts for details.
   */
  constructor(public readonly raw_txn: RawTransaction, public readonly authenticator: TransactionAuthenticator) {}

  serialize(serializer: Serializer): void {
    this.raw_txn.serialize(serializer);
    this.authenticator.serialize(serializer);
  }

  static deserialize(deserializer: Deserializer): SignedTransaction {
    const raw_txn = RawTransaction.deserialize(deserializer);
    const authenticator = TransactionAuthenticator.deserialize(deserializer);
    return new SignedTransaction(raw_txn, authenticator);
  }
}

export abstract class TransactionPayload {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionPayload {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionPayloadVariantWriteSet.load(deserializer);
      case 1:
        return TransactionPayloadVariantScript.load(deserializer);
      case 2:
        return TransactionPayloadVariantModuleBundle.load(deserializer);
      case 3:
        return TransactionPayloadVariantScriptFunction.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionPayload: ${index}`);
    }
  }
}

export class TransactionPayloadVariantWriteSet extends TransactionPayload {
  serialize(serializer: Serializer): void {
    throw new Error("Not implemented");
  }

  static load(deserializer: Deserializer): TransactionPayloadVariantWriteSet {
    throw new Error("Not implemented");
  }
}

export class TransactionPayloadVariantScript extends TransactionPayload {
  constructor(public readonly value: Script) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadVariantScript {
    const value = Script.deserialize(deserializer);
    return new TransactionPayloadVariantScript(value);
  }
}

export class TransactionPayloadVariantModuleBundle extends TransactionPayload {
  constructor(public readonly value: ModuleBundle) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadVariantModuleBundle {
    const value = ModuleBundle.deserialize(deserializer);
    return new TransactionPayloadVariantModuleBundle(value);
  }
}

export class TransactionPayloadVariantScriptFunction extends TransactionPayload {
  constructor(public readonly value: ScriptFunction) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionPayloadVariantScriptFunction {
    const value = ScriptFunction.deserialize(deserializer);
    return new TransactionPayloadVariantScriptFunction(value);
  }
}

export class ChainId {
  constructor(public readonly value: uint8) {}

  serialize(serializer: Serializer): void {
    serializer.serializeU8(this.value);
  }

  static deserialize(deserializer: Deserializer): ChainId {
    const value = deserializer.deserializeU8();
    return new ChainId(value);
  }
}

export abstract class TransactionArgument {
  abstract serialize(serializer: Serializer): void;

  static deserialize(deserializer: Deserializer): TransactionArgument {
    const index = deserializer.deserializeUleb128AsU32();
    switch (index) {
      case 0:
        return TransactionArgumentVariantU8.load(deserializer);
      case 1:
        return TransactionArgumentVariantU64.load(deserializer);
      case 2:
        return TransactionArgumentVariantU128.load(deserializer);
      case 3:
        return TransactionArgumentVariantAddress.load(deserializer);
      case 4:
        return TransactionArgumentVariantU8Vector.load(deserializer);
      case 5:
        return TransactionArgumentVariantBool.load(deserializer);
      default:
        throw new Error(`Unknown variant index for TransactionArgument: ${index}`);
    }
  }
}

export class TransactionArgumentVariantU8 extends TransactionArgument {
  constructor(public readonly value: uint8) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(0);
    serializer.serializeU8(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentVariantU8 {
    const value = deserializer.deserializeU8();
    return new TransactionArgumentVariantU8(value);
  }
}

export class TransactionArgumentVariantU64 extends TransactionArgument {
  constructor(public readonly value: uint64) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(1);
    serializer.serializeU64(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentVariantU64 {
    const value = deserializer.deserializeU64();
    return new TransactionArgumentVariantU64(value);
  }
}

export class TransactionArgumentVariantU128 extends TransactionArgument {
  constructor(public readonly value: uint128) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(2);
    serializer.serializeU128(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentVariantU128 {
    const value = deserializer.deserializeU128();
    return new TransactionArgumentVariantU128(value);
  }
}

export class TransactionArgumentVariantAddress extends TransactionArgument {
  constructor(public readonly value: AccountAddress) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(3);
    this.value.serialize(serializer);
  }

  static load(deserializer: Deserializer): TransactionArgumentVariantAddress {
    const value = AccountAddress.deserialize(deserializer);
    return new TransactionArgumentVariantAddress(value);
  }
}

export class TransactionArgumentVariantU8Vector extends TransactionArgument {
  constructor(public readonly value: bytes) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(4);
    serializer.serializeBytes(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentVariantU8Vector {
    const value = deserializer.deserializeBytes();
    return new TransactionArgumentVariantU8Vector(value);
  }
}

export class TransactionArgumentVariantBool extends TransactionArgument {
  constructor(public readonly value: bool) {
    super();
  }

  serialize(serializer: Serializer): void {
    serializer.serializeU32AsUleb128(5);
    serializer.serializeBool(this.value);
  }

  static load(deserializer: Deserializer): TransactionArgumentVariantBool {
    const value = deserializer.deserializeBool();
    return new TransactionArgumentVariantBool(value);
  }
}

export function serializeVectorBytes(value: Seq<bytes>, serializer: Serializer): void {
  serializer.serializeU32AsUleb128(value.length);
  value.forEach((item: bytes) => {
    serializer.serializeBytes(item);
  });
}

export function deserializeVectorBytes(deserializer: Deserializer): Seq<bytes> {
  const length = deserializer.deserializeUleb128AsU32();
  const list: Seq<bytes> = [];
  for (let i = 0; i < length; i++) {
    list.push(deserializer.deserializeBytes());
  }
  return list;
}
