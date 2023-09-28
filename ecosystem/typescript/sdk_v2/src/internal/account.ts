/**
 * This file contains the underlying implementations for exposed API surface in
 * the {@link api/account}. By moving the methods out into a separate file,
 * other namespaces and processes can access these methods without depending on the entire
 * account namespace and without having a dependency cycle error.
 */

import {AptosConfig} from "../api/aptos_config";
import {get, getFullNode} from "../client";
import {paginateWithCursor} from "../utils/paginate_with_cursor";
import {AccountAddress, Hex} from "../core";
import {AptosApiType} from "../utils/const";
import {queryIndexer} from "./general";
import {
  AccountData,
  GetAccountCoinsCountResponse,
  GetAccountCoinsDataResponse,
  GetAccountCollectionsWithOwnedTokenResponse,
  GetAccountOwnedObjectsResponse,
  GetAccountOwnedTokensFromCollectionResponse,
  GetAccountOwnedTokensQueryResponse,
  GetAccountTokensCountQueryResponse,
  GetAccountTransactionsCountResponse,
  HexInput,
  IndexerPaginationArgs,
  LedgerVersion,
  MoveModuleBytecode,
  MoveResource,
  MoveResourceType,
  OrderBy,
  PaginationArgs,
  TokenStandard,
  TransactionResponse,
} from "../types";
import {
  GetAccountCoinsCountQuery,
  GetAccountCoinsDataQuery,
  GetAccountCollectionsWithOwnedTokensQuery,
  GetAccountOwnedObjectsQuery,
  GetAccountOwnedTokensFromCollectionQuery,
  GetAccountOwnedTokensQuery,
  GetAccountTokensCountQuery,
  GetAccountTransactionsCountQuery,
} from "../types/generated/operations";
import {
  GetAccountCoinsCount,
  GetAccountCoinsData,
  GetAccountCollectionsWithOwnedTokens,
  GetAccountOwnedObjects,
  GetAccountOwnedTokens,
  GetAccountOwnedTokensFromCollection,
  GetAccountTokensCount,
  GetAccountTransactionsCount,
} from "../types/generated/queries";

export async function getInfo(args: { aptosConfig: AptosConfig; accountAddress: HexInput }): Promise<AccountData> {
  const { aptosConfig, accountAddress } = args;
  const { data } = await getFullNode<{}, AccountData>(
    {
      aptosConfig,
      name: "getInfo",
      path: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}`,
    },
  );
  return data;
}

export async function getModules(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: PaginationArgs & LedgerVersion;
}): Promise<MoveModuleBytecode[]> {
  const { aptosConfig, accountAddress, options } = args;
  const data = await paginateWithCursor<{}, MoveModuleBytecode[]>(
    {
      aptosConfig,
      name: "getModules",
      path: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/modules`,
      params: { ledger_version: options?.ledgerVersion, start: options?.start, limit: options?.limit ?? 1000 },
    },
  );
  return data;
}

/**
 * Queries for a move module given account address and module name
 *
 * @param accountAddress Hex-encoded 32 byte Aptos account address
 * @param moduleName The name of the module
 * @param query.ledgerVersion Specifies ledger version of transactions. By default latest version will be used
 * @returns The move module.
 */
export async function getModule(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  moduleName: string;
  options?: LedgerVersion;
}): Promise<MoveModuleBytecode> {
  const { aptosConfig, accountAddress, moduleName, options } = args;
  const { data } = await getFullNode<{}, MoveModuleBytecode>(
    {
      aptosConfig,
      name: "getModule",
      path: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/module/${moduleName}`,
      params: { ledger_version: options?.ledgerVersion },
    },
  );
  return data;
}

export async function getTransactions(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: PaginationArgs;
}): Promise<TransactionResponse[]> {
  const { aptosConfig, accountAddress, options } = args;
  const data = await paginateWithCursor<{}, TransactionResponse[]>(
    {
      aptosConfig,
      name: "getTransactions",
      path: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/transactions`,
      params: { start: options?.start, limit: options?.limit },
    },
  );
  return data;
}

export async function getResources(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: PaginationArgs & LedgerVersion;
}): Promise<MoveResource[]> {
  const { aptosConfig, accountAddress, options } = args;
  const data = await paginateWithCursor<{}, MoveResource[]>(
    {
      aptosConfig,
      name: "getResources",
      path: `accounts/${AccountAddress.fromHexInput({ input: accountAddress }).toString()}/resources`,
      params: { ledger_version: options?.ledgerVersion, start: options?.start, limit: options?.limit ?? 999 },
    },
    aptosConfig,
  );
  return data;
}

export async function getResource(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  resourceType: MoveResourceType;
  options?: LedgerVersion;
}): Promise<MoveResource> {
  const { aptosConfig, accountAddress, resourceType, options } = args;
  const { data } = await getFullNode<{}, MoveResource>(
    {
      aptosConfig,
      name: "getResource",
      path: `accounts/${AccountAddress.fromHexInput({
        input: accountAddress,
      }).toString()}/resource/${resourceType}`,
      params: { ledger_version: options?.ledgerVersion },
    },
  );
  return data;
}

export async function getAccountTokensCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
}): Promise<GetAccountTokensCountQueryResponse> {
  const { aptosConfig, accountAddress } = args;

  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: address },
    amount: { _gt: "0" },
  };

  const graphqlQuery = {
    query: GetAccountTokensCount,
    variables: { where_condition: whereCondition },
  };

  const data = await queryIndexer<GetAccountTokensCountQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountTokensCount",
  });

  return data.current_token_ownerships_v2_aggregate.aggregate;
}

export async function getAccountOwnedTokens(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: {
    tokenStandard?: TokenStandard;
    pagination?: IndexerPaginationArgs;
    orderBy?: OrderBy<GetAccountOwnedTokensQueryResponse[0]>;
  };
}): Promise<GetAccountOwnedTokensQueryResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: address },
    amount: { _gt: 0 },
  };

  if (options?.tokenStandard) {
    whereCondition.token_standard = { _eq: options?.tokenStandard };
  }

  const graphqlQuery = {
    query: GetAccountOwnedTokens,
    variables: {
      where_condition: whereCondition,
      offset: options?.pagination?.offset,
      limit: options?.pagination?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountOwnedTokensQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountOwnedTokens",
  });

  return data.current_token_ownerships_v2;
}

export async function getAccountOwnedTokensFromCollectionAddress(args: {
  aptosConfig: AptosConfig;
  ownerAddress: HexInput;
  collectionAddress: HexInput;
  options?: {
    tokenStandard?: TokenStandard;
    pagination?: IndexerPaginationArgs;
    orderBy?: OrderBy<GetAccountOwnedTokensFromCollectionResponse[0]>;
  };
}): Promise<GetAccountOwnedTokensFromCollectionResponse> {
  const { aptosConfig, ownerAddress, collectionAddress, options } = args;
  const accountAddress = AccountAddress.fromHexInput({ input: ownerAddress }).toString();
  const collAddress = Hex.fromHexInput({ hexInput: collectionAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: accountAddress },
    current_token_data: { collection_id: { _eq: collAddress } },
    amount: { _gt: 0 },
  };

  if (options?.tokenStandard) {
    whereCondition.token_standard = { _eq: options?.tokenStandard };
  }

  const graphqlQuery = {
    query: GetAccountOwnedTokensFromCollection,
    variables: {
      where_condition: whereCondition,
      offset: options?.pagination?.offset,
      limit: options?.pagination?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountOwnedTokensFromCollectionQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountOwnedTokensFromCollectionAddress",
  });

  return data.current_token_ownerships_v2;
}

export async function getAccountCollectionsWithOwnedTokens(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: {
    tokenStandard?: TokenStandard;
    pagination?: IndexerPaginationArgs;
    orderBy?: OrderBy<GetAccountCollectionsWithOwnedTokenResponse[0]>;
  };
}): Promise<GetAccountCollectionsWithOwnedTokenResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: address },
    amount: { _gt: 0 },
  };

  if (options?.tokenStandard) {
    whereCondition.current_collection = { token_standard: { _eq: options?.tokenStandard } };
  }

  const graphqlQuery = {
    query: GetAccountCollectionsWithOwnedTokens,
    variables: {
      where_condition: whereCondition,
      offset: options?.pagination?.offset,
      limit: options?.pagination?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountCollectionsWithOwnedTokensQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountCollectionsWithOwnedTokens",
  });

  return data.current_collection_ownership_v2_view;
}

export async function getAccountTransactionsCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
}): Promise<GetAccountTransactionsCountResponse> {
  const { aptosConfig, accountAddress } = args;

  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const graphqlQuery = {
    query: GetAccountTransactionsCount,
    variables: { address },
  };

  const data = await queryIndexer<GetAccountTransactionsCountQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountTransactionsCount",
  });

  return data.account_transactions_aggregate.aggregate;
}

export async function getAccountCoinsData(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
  options?: {
    pagination?: IndexerPaginationArgs;
    orderBy?: OrderBy<GetAccountCoinsDataResponse[0]>;
  };
}): Promise<GetAccountCoinsDataResponse> {
  const { aptosConfig, accountAddress, options } = args;
  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: address },
  };

  const graphqlQuery = {
    query: GetAccountCoinsData,
    variables: {
      where_condition: whereCondition,
      offset: options?.pagination?.offset,
      limit: options?.pagination?.limit,
      order_by: options?.orderBy,
    },
  };

  const data = await queryIndexer<GetAccountCoinsDataQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountCoinsData",
  });

  return data.current_fungible_asset_balances;
}

export async function getAccountCoinsCount(args: {
  aptosConfig: AptosConfig;
  accountAddress: HexInput;
}): Promise<GetAccountCoinsCountResponse> {
  const { aptosConfig, accountAddress } = args;
  const address = AccountAddress.fromHexInput({ input: accountAddress }).toString();

  const graphqlQuery = {
    query: GetAccountCoinsCount,
    variables: { address },
  };

  const data = await queryIndexer<GetAccountCoinsCountQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountCoinsCount",
  });

  return data.current_fungible_asset_balances_aggregate.aggregate;
}

export async function getAccountOwnedObjects(args: {
  aptosConfig: AptosConfig;
  ownerAddress: HexInput;
  options?: {
    pagination?: IndexerPaginationArgs;
    orderBy?: OrderBy<GetAccountOwnedObjectsResponse[0]>;
  };
}): Promise<GetAccountOwnedObjectsResponse> {
  const { aptosConfig, ownerAddress, options } = args;
  const address = AccountAddress.fromHexInput({ input: ownerAddress }).toString();

  const whereCondition: any = {
    owner_address: { _eq: address },
  };
  const graphqlQuery = {
    query: GetAccountOwnedObjects,
    variables: {
      where_condition: whereCondition,
      offset: options?.pagination?.offset,
      limit: options?.pagination?.limit,
      order_by: options?.orderBy,
    },
  };
  const data = await queryIndexer<GetAccountOwnedObjectsQuery>({
    aptosConfig,
    query: graphqlQuery,
    name: "getAccountOwnedObjects",
  });

  return data.current_objects;
}
