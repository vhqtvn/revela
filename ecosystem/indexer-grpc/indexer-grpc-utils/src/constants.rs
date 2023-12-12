// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// The maximum number of transactions that can be stored in a blob.
pub const BLOB_STORAGE_SIZE: usize = 1_000;
/// GRPC request metadata key for the token ID.
pub const GRPC_AUTH_TOKEN_HEADER: &str = "x-aptos-data-authorization";
/// GRPC request metadata key for the request name. This is used to identify the
/// data destination.
pub const GRPC_REQUEST_NAME_HEADER: &str = "x-aptos-request-name";
pub const GRPC_API_GATEWAY_API_KEY_HEADER: &str = "authorization";
// Limit the message size to 15MB. By default the downstream can receive up to 15MB.
pub const MESSAGE_SIZE_LIMIT: usize = 1024 * 1024 * 15;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct IndexerGrpcRequestMetadata {
    pub processor_name: String,
    pub request_email: String,
    pub request_user_classification: String,
    pub request_api_key_name: String,
    pub request_connection_id: String,
    // Token is no longer needed behind api gateway.
    #[deprecated]
    pub request_token: String,
}
