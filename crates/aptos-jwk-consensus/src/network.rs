// Copyright © Aptos Foundation

use crate::{
    network_interface::{JWKConsensusNetworkClient, RPC},
    types::JWKConsensusMsg,
};
use anyhow::bail;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::network_id::NetworkId;
use aptos_consensus_types::common::Author;
#[cfg(test)]
use aptos_infallible::RwLock;
use aptos_logger::warn;
use aptos_network::{
    application::interface::{NetworkClient, NetworkServiceEvents},
    protocols::network::{Event, RpcError},
    ProtocolId,
};
use aptos_reliable_broadcast::RBNetworkSender;
use aptos_types::account_address::AccountAddress;
use bytes::Bytes;
use futures::Stream;
use futures_channel::oneshot;
use futures_util::{
    stream::{select, select_all, StreamExt},
    SinkExt,
};
#[cfg(test)]
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

pub struct IncomingRpcRequest {
    pub msg: JWKConsensusMsg,
    pub sender: AccountAddress,
    pub response_sender: Box<dyn RpcResponseSender>,
}

pub struct NetworkSender {
    author: AccountAddress,
    jwk_network_client: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,
    self_sender: aptos_channels::Sender<Event<JWKConsensusMsg>>,
}

impl NetworkSender {
    pub fn new(
        author: AccountAddress,
        jwk_network_client: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,
        self_sender: aptos_channels::Sender<Event<JWKConsensusMsg>>,
    ) -> Self {
        Self {
            author,
            jwk_network_client,
            self_sender,
        }
    }
}

#[async_trait::async_trait]
impl RBNetworkSender<JWKConsensusMsg> for NetworkSender {
    async fn send_rb_rpc(
        &self,
        receiver: Author,
        msg: JWKConsensusMsg,
        time_limit: Duration,
    ) -> anyhow::Result<JWKConsensusMsg> {
        if receiver == self.author {
            let (tx, rx) = oneshot::channel();
            let self_msg = Event::RpcRequest(receiver, msg, RPC[0], tx);
            self.self_sender.clone().send(self_msg).await?;
            if let Ok(Ok(Ok(bytes))) = timeout(time_limit, rx).await {
                Ok(RPC[0].from_bytes(&bytes)?)
            } else {
                bail!("self rpc failed");
            }
        } else {
            let result = self
                .jwk_network_client
                .send_rpc(receiver, msg, time_limit)
                .await?;
            Ok(result)
        }
    }
}

pub trait RpcResponseSender: Send + Sync {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>);
}

pub struct RealRpcResponseSender {
    pub inner: Option<oneshot::Sender<Result<Bytes, RpcError>>>,
    pub protocol: ProtocolId,
}

impl RpcResponseSender for RealRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>) {
        let rpc_response = response
            .and_then(|msg| self.protocol.to_bytes(&msg).map(Bytes::from))
            .map_err(RpcError::ApplicationError);
        if let Some(tx) = self.inner.take() {
            let _ = tx.send(rpc_response);
        }
    }
}

#[cfg(test)]
pub struct DummyRpcResponseSender {
    pub rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>,
}

#[cfg(test)]
impl DummyRpcResponseSender {
    pub fn new(rpc_response_collector: Arc<RwLock<Vec<anyhow::Result<JWKConsensusMsg>>>>) -> Self {
        Self {
            rpc_response_collector,
        }
    }
}

#[cfg(test)]
impl RpcResponseSender for DummyRpcResponseSender {
    fn send(&mut self, response: anyhow::Result<JWKConsensusMsg>) {
        self.rpc_response_collector.write().push(response);
    }
}

pub struct NetworkReceivers {
    pub rpc_rx: aptos_channel::Receiver<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

pub struct NetworkTask {
    all_events: Box<dyn Stream<Item = Event<JWKConsensusMsg>> + Send + Unpin>,
    rpc_tx: aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>,
}

impl NetworkTask {
    /// Establishes the initial connections with the peers and returns the receivers.
    pub fn new(
        network_service_events: NetworkServiceEvents<JWKConsensusMsg>,
        self_receiver: aptos_channels::Receiver<Event<JWKConsensusMsg>>,
    ) -> (NetworkTask, NetworkReceivers) {
        let (rpc_tx, rpc_rx) = aptos_channel::new(QueueStyle::FIFO, 10, None);

        let network_and_events = network_service_events.into_network_and_events();
        if (network_and_events.values().len() != 1)
            || !network_and_events.contains_key(&NetworkId::Validator)
        {
            panic!("The network has not been setup correctly for JWK consensus!");
        }

        // Collect all the network events into a single stream
        let network_events: Vec<_> = network_and_events.into_values().collect();
        let network_events = select_all(network_events).fuse();
        let all_events = Box::new(select(network_events, self_receiver));

        (NetworkTask { rpc_tx, all_events }, NetworkReceivers {
            rpc_rx,
        })
    }

    pub async fn start(mut self) {
        while let Some(message) = self.all_events.next().await {
            match message {
                Event::RpcRequest(peer_id, msg, protocol, response_sender) => {
                    let req = IncomingRpcRequest {
                        msg,
                        sender: peer_id,
                        response_sender: Box::new(RealRpcResponseSender {
                            inner: Some(response_sender),
                            protocol,
                        }),
                    };

                    if let Err(e) = self.rpc_tx.push(peer_id, (peer_id, req)) {
                        warn!(error = ?e, "aptos channel closed");
                    };
                },
                _ => {
                    // Ignore
                },
            }
        }
    }
}
