// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::serializer::{
    ExecutionCorrectnessInput, SerializerClient, SerializerService, TSerializerClient,
};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_secure_net::{NetworkClient, NetworkServer};

use aptos_vm::AptosVM;
use executor::block_executor::BlockExecutor;
use executor_types::Error;
use std::net::SocketAddr;
use storage_client::StorageClient;
use storage_interface::DbReaderWriter;

pub trait RemoteService {
    fn client(&self) -> SerializerClient {
        let network_client =
            NetworkClient::new("execution", self.server_address(), self.network_timeout());
        let service = Box::new(RemoteClient::new(Mutex::new(network_client)));
        SerializerClient::new_client(service)
    }

    fn server_address(&self) -> SocketAddr;
    fn network_timeout(&self) -> u64;
}

pub fn execute(
    storage_addr: SocketAddr,
    listen_addr: SocketAddr,
    prikey: Option<Ed25519PrivateKey>,
    network_timeout: u64,
) {
    let block_executor = Box::new(BlockExecutor::<AptosVM>::new(DbReaderWriter::new(
        StorageClient::new(&storage_addr, network_timeout),
    )));
    let serializer_service = SerializerService::new(block_executor, prikey);
    let mut network_server = NetworkServer::new("execution", listen_addr, network_timeout);

    loop {
        if let Err(e) = process_one_message(&mut network_server, &serializer_service) {
            warn!("Warning: Failed to process message: {}", e);
        }
    }
}

fn process_one_message(
    network_server: &mut NetworkServer,
    serializer_service: &SerializerService,
) -> Result<(), Error> {
    let request = network_server.read()?;
    let response = serializer_service.handle_message(request)?;
    network_server.write(&response)?;
    Ok(())
}

struct RemoteClient {
    network_client: Mutex<NetworkClient>,
}

impl RemoteClient {
    pub fn new(network_client: Mutex<NetworkClient>) -> Self {
        Self { network_client }
    }

    fn process_one_message(&self, input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut client = self.network_client.lock();
        client.write(input)?;
        client.read().map_err(|e| e.into())
    }
}

impl TSerializerClient for RemoteClient {
    fn request(&self, input: ExecutionCorrectnessInput) -> Result<Vec<u8>, Error> {
        let input_message = bcs::to_bytes(&input)?;
        loop {
            match self.process_one_message(&input_message) {
                Err(err) => warn!("Failed to communicate with LEC service: {}", err),
                Ok(value) => return Ok(value),
            }
        }
    }
}
