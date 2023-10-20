// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(unused)]

use aptos_logger::debug;
use http::header::{HeaderName, HeaderValue};
use hyper::{Body, Response, StatusCode};
use std::{convert::Into, iter::IntoIterator};

pub const UNEXPECTED_ERROR_MESSAGE: &str = "An unexpected error was encountered!";

pub(crate) fn reply_with_status<T>(status_code: StatusCode, message: T) -> Response<Body>
where
    T: Into<Body>,
{
    reply_with_internal(status_code, [], message)
}

pub(crate) fn reply_with<H, T>(headers: H, body: T) -> Response<Body>
where
    H: IntoIterator<Item = (HeaderName, HeaderValue)>,
    T: Into<Body>,
{
    reply_with_internal(StatusCode::OK, headers, body)
}

fn reply_with_internal<T, H>(status_code: StatusCode, headers: H, body: T) -> Response<Body>
where
    H: IntoIterator<Item = (HeaderName, HeaderValue)>,
    T: Into<Body>,
{
    let mut builder = Response::builder().status(status_code);
    for (header_name, header_value) in headers {
        builder = builder.header(header_name, header_value);
    }

    builder.body(body.into()).unwrap_or_else(|e| {
        debug!("Error encountered when generating response: {:?}", e);
        let mut response = Response::new(Body::from(UNEXPECTED_ERROR_MESSAGE));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    })
}
