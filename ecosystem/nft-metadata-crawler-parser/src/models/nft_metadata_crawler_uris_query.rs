// Copyright © Aptos Foundation

use crate::{
    schema::nft_metadata_crawler::parsed_token_uris, utils::constants::MAX_RETRY_TIME_SECONDS,
};
use backoff::{retry, ExponentialBackoff};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(primary_key(token_uri))]
#[diesel(table_name = parsed_token_uris)]
pub struct NFTMetadataCrawlerURIsQuery {
    pub token_uri: String,
    pub raw_image_uri: Option<String>,
    pub raw_animation_uri: Option<String>,
    pub cdn_json_uri: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub cdn_animation_uri: Option<String>,
    pub json_parser_retry_count: i32,
    pub image_optimizer_retry_count: i32,
    pub animation_optimizer_retry_count: i32,
    pub inserted_at: chrono::NaiveDateTime,
}

impl NFTMetadataCrawlerURIsQuery {
    pub fn get_by_token_uri(
        token_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Option<Self> {
        let mut op = || {
            parsed_token_uris::table
                .find(token_uri.clone())
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
                .map_err(Into::into)
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        retry(backoff, &mut op).expect("Querying token_uri should not fail")
    }

    pub fn get_by_raw_image_uri(
        token_uri: String,
        raw_image_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Option<Self> {
        let mut op = || {
            parsed_token_uris::table
                .filter(parsed_token_uris::raw_image_uri.eq(raw_image_uri.clone()))
                .filter(parsed_token_uris::token_uri.ne(token_uri.clone()))
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
                .map_err(Into::into)
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        retry(backoff, &mut op).expect("Querying raw_image_uri should not fail")
    }

    pub fn get_by_raw_animation_uri(
        token_uri: String,
        raw_animation_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Option<Self> {
        let mut op = || {
            parsed_token_uris::table
                .filter(parsed_token_uris::raw_animation_uri.eq(raw_animation_uri.clone()))
                .filter(parsed_token_uris::token_uri.ne(token_uri.clone()))
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
                .map_err(Into::into)
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        retry(backoff, &mut op).expect("Querying raw_animation_uri should not fail")
    }
}
