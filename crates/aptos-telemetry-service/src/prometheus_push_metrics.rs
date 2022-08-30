// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auth::with_auth, context::Context, types::auth::Claims};
use aptos_config::config::PeerRole;
use aptos_logger::{debug, error};
use reqwest::StatusCode;
use warp::{filters::BoxedFilter, hyper::body::Bytes, reply, Filter, Rejection, Reply};

pub fn metrics_ingest(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("push-metrics")
        .and(warp::post())
        .and(context.clone().filter())
        .and(with_auth(
            context,
            vec![PeerRole::Validator, PeerRole::ValidatorFullNode],
        ))
        .and(warp::body::bytes())
        .and_then(handle_metrics_ingest)
        .boxed()
}

pub async fn handle_metrics_ingest(
    context: Context,
    claims: Claims,
    metrics_body: Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    let extra_labels = claims_to_extra_labels(&claims);

    let res = context
        .victoria_metrics_client
        .unwrap()
        .post_prometheus_metrics(metrics_body, extra_labels)
        .await;

    match res {
        Ok(res) => {
            if res.status().is_success() {
                debug!("remote write to victoria metrics succeeded");
            } else {
                error!(
                    "remote write failed to victoria_metrics: {}",
                    res.error_for_status().err().unwrap()
                );
            }
        }
        Err(err) => {
            error!("error sending remote write request: {}", err);
        }
    }

    Ok(reply::with_status(reply::reply(), StatusCode::CREATED))
}

fn claims_to_extra_labels(claims: &Claims) -> Vec<String> {
    vec![
        format!("role={}", claims.peer_role),
        format!("chain_name={}", claims.chain_id),
        format!("namespace={}", "telemetry-service"),
        // for community nodes we cannot determine which pod name they run in (or whether they run in k8s at all), so we use the peer id as an approximation/replacement for pod_name
        // This works well with our existing grafana dashboards
        format!(
            "kubernetes_pod_name=peer_id:{}",
            claims.peer_id.to_hex_literal()
        ),
    ]
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use aptos_types::{chain_id::ChainId, PeerId};
    #[test]
    fn verify_labels() {
        let claims = claims_to_extra_labels(&super::Claims {
            chain_id: ChainId::new(25),
            peer_id: PeerId::from_str("0x1").unwrap(),
            peer_role: PeerRole::Validator,
            epoch: 3,
            exp: 123,
            iat: 123,
        });
        assert_eq!(
            claims,
            vec![
                "role=validator",
                "chain_name=25",
                "namespace=telemetry-service",
                "kubernetes_pod_name=peer_id:0x1",
            ]
        );
    }
}
