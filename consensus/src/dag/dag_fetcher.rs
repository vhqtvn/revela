// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{dag_network::RpcWithFallback, types::NodeMetadata, RpcHandler};
use crate::dag::{
    dag_network::DAGNetworkSender,
    dag_store::Dag,
    types::{CertifiedNode, FetchResponse, Node, RemoteFetchRequest},
};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_time_service::TimeService;
use aptos_types::epoch_state::EpochState;
use futures::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};
use thiserror::Error as ThisError;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

pub enum LocalFetchRequest {
    Node(Node, oneshot::Sender<Node>),
    CertifiedNode(CertifiedNode, oneshot::Sender<CertifiedNode>),
}

impl LocalFetchRequest {
    pub fn responders(&self, validators: &[Author]) -> Vec<Author> {
        match self {
            LocalFetchRequest::Node(node, _) => vec![*node.author()],
            LocalFetchRequest::CertifiedNode(node, _) => {
                node.signatures().get_signers_addresses(validators)
            },
        }
    }

    pub fn notify(self) {
        if match self {
            LocalFetchRequest::Node(node, sender) => sender.send(node).map_err(|_| ()),
            LocalFetchRequest::CertifiedNode(node, sender) => sender.send(node).map_err(|_| ()),
        }
        .is_err()
        {
            error!("Failed to send node back");
        }
    }

    pub fn node(&self) -> &Node {
        match self {
            LocalFetchRequest::Node(node, _) => node,
            LocalFetchRequest::CertifiedNode(node, _) => node,
        }
    }
}

struct DagFetcher {
    epoch_state: Arc<EpochState>,
    network: Arc<dyn DAGNetworkSender>,
    dag: Arc<RwLock<Dag>>,
    request_rx: Receiver<LocalFetchRequest>,
    time_service: TimeService,
}

impl DagFetcher {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn DAGNetworkSender>,
        dag: Arc<RwLock<Dag>>,
        time_service: TimeService,
    ) -> (Self, Sender<LocalFetchRequest>) {
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(16);
        (
            Self {
                epoch_state,
                network,
                dag,
                request_rx,
                time_service,
            },
            request_tx,
        )
    }

    pub async fn start(mut self) {
        while let Some(local_request) = self.request_rx.recv().await {
            let responders = local_request
                .responders(&self.epoch_state.verifier.get_ordered_account_addresses());
            let remote_request = {
                let dag_reader = self.dag.read();

                let missing_parents: Vec<NodeMetadata> = dag_reader
                    .filter_missing(local_request.node().parents_metadata())
                    .cloned()
                    .collect();

                if missing_parents.is_empty() {
                    local_request.notify();
                    continue;
                }

                let target = local_request.node();
                RemoteFetchRequest::new(
                    target.metadata().epoch(),
                    missing_parents,
                    dag_reader.bitmask(local_request.node().round()),
                )
            };

            let mut rpc = RpcWithFallback::new(
                responders,
                remote_request.clone().into(),
                Duration::from_millis(500),
                Duration::from_secs(1),
                self.network.clone(),
                self.time_service.clone(),
            );
            while let Some(response) = rpc.next().await {
                if let Ok(response) =
                    response
                        .and_then(FetchResponse::try_from)
                        .and_then(|response| {
                            response.verify(&remote_request, &self.epoch_state.verifier)
                        })
                {
                    let certified_nodes = response.certified_nodes();
                    // TODO: support chunk response or fallback to state sync
                    {
                        let mut dag_writer = self.dag.write();
                        for node in certified_nodes {
                            if let Err(e) = dag_writer.add_node(node) {
                                error!("Failed to add node {}", e);
                            }
                        }
                    }

                    if self
                        .dag
                        .read()
                        .all_exists(local_request.node().parents_metadata())
                    {
                        local_request.notify();
                        break;
                    }
                }
            }
            // TODO retry
        }
    }
}

#[derive(Debug, ThisError)]
pub enum FetchRequestHandleError {
    #[error("parents are missing")]
    ParentsMissing,
}

pub struct FetchRequestHandler {
    dag: Arc<RwLock<Dag>>,
    author_to_index: HashMap<Author, usize>,
}

impl FetchRequestHandler {
    pub fn new(dag: Arc<RwLock<Dag>>, epoch_state: Arc<EpochState>) -> Self {
        Self {
            dag,
            author_to_index: epoch_state.verifier.address_to_validator_index().clone(),
        }
    }
}

impl RpcHandler for FetchRequestHandler {
    type Request = RemoteFetchRequest;
    type Response = FetchResponse;

    fn process(&mut self, message: Self::Request) -> anyhow::Result<Self::Response> {
        let dag_reader = self.dag.read();

        // `Certified Node`: In the good case, there should exist at least one honest validator that
        // signed the Certified Node that has the all the parents to fulfil this
        // request.
        // `Node`: In the good case, the sender of the Node should have the parents in its local DAG
        // to satisfy this request.
        ensure!(
            dag_reader.all_exists(message.targets().iter()),
            FetchRequestHandleError::ParentsMissing
        );

        let certified_nodes: Vec<_> = dag_reader
            .reachable(
                message.targets(),
                Some(message.exists_bitmask().first_round()),
                |_| true,
            )
            .filter_map(|node_status| {
                let arc_node = node_status.as_node();
                self.author_to_index
                    .get(arc_node.author())
                    .and_then(|author_idx| {
                        if !message.exists_bitmask().has(arc_node.round(), *author_idx) {
                            Some(arc_node.as_ref().clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();

        // TODO: decide if the response is too big and act accordingly.

        Ok(FetchResponse::new(message.epoch(), certified_nodes))
    }
}
