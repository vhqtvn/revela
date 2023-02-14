// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
#[cfg(any(test, feature = "fuzzing"))]
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A struct that represents a globally unique id for an Event stream that a user can listen to.
/// By design, the lower part of EventKey is the same as account address.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct EventKey {
    creation_number: u64,
    account_address: AccountAddress,
}

impl EventKey {
    pub fn new(creation_number: u64, account_address: AccountAddress) -> Self {
        Self {
            creation_number,
            account_address,
        }
    }

    /// Convert event key into a byte array.
    pub fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).unwrap()
    }

    /// Get the account address part in this event key
    pub fn get_creator_address(&self) -> AccountAddress {
        self.account_address
    }

    /// If this is the `ith` EventKey` created by `get_creator_address()`, return `i`
    pub fn get_creation_number(&self) -> u64 {
        self.creation_number
    }

    #[cfg(any(test, feature = "fuzzing"))]
    /// Create a random event key for testing
    pub fn random() -> Self {
        let mut rng = OsRng;
        let salt = rng.next_u64();
        EventKey::new(salt, AccountAddress::random())
    }
    /*
    pub fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, EventKeyParseError> {
        <[u8; Self::LENGTH]>::from_hex(hex)
            .map_err(|_| EventKeyParseError)
            .map(Self)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self, EventKeyParseError> {
        <[u8; Self::LENGTH]>::try_from(bytes.as_ref())
            .map_err(|_| EventKeyParseError)
            .map(Self)
    }
    */
}

/*
impl FromStr for EventKey {
    type Err = EventKeyParseError;

    fn from_str(s: &str) -> Result<Self, EventKeyParseError> {
        EventKey::from_hex(s)
    }
}
*/

/*
impl From<EventKey> for [u8; EventKey::LENGTH] {
    fn from(event_key: EventKey) -> Self {
        event_key.0
    }
}

impl From<&EventKey> for [u8; EventKey::LENGTH] {
    fn from(event_key: &EventKey) -> Self {
        event_key.0
    }
}
*/

impl fmt::LowerHex for EventKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "0x")?;
        }

        for byte in self.to_bytes() {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}

impl fmt::Display for EventKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:x}", self)
    }
}

/*
impl TryFrom<&[u8]> for EventKey {
    type Error = EventKeyParseError;

    /// Tries to convert the provided byte array into Event Key.
    fn try_from(bytes: &[u8]) -> Result<EventKey, EventKeyParseError> {
        Self::from_bytes(bytes)
    }
}
*/

#[derive(Clone, Copy, Debug)]
pub struct EventKeyParseError;

impl fmt::Display for EventKeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        write!(f, "unable to parse EventKey")
    }
}

impl std::error::Error for EventKeyParseError {}

/// A Rust representation of an Event Handle Resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventHandle {
    /// Number of events in the event stream.
    count: u64,
    /// The associated globally unique key that is used as the key to the EventStore.
    key: EventKey,
}

impl EventHandle {
    /// Constructs a new Event Handle
    pub fn new(key: EventKey, count: u64) -> Self {
        EventHandle { count, key }
    }

    /// Return the key to where this event is stored in EventStore.
    pub fn key(&self) -> &EventKey {
        &self.key
    }

    /// Return the counter for the handle
    pub fn count(&self) -> u64 {
        self.count
    }

    #[cfg(any(test, feature = "fuzzing"))]
    /// Create a random event key for testing
    pub fn random(count: u64) -> Self {
        Self {
            key: EventKey::random(),
            count,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn count_mut(&mut self) -> &mut u64 {
        &mut self.count
    }
}
