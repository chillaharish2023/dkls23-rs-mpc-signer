//! MPC coordination utilities

use crate::{PartyId, Result, SessionId};
use serde::{de::DeserializeOwned, Serialize};

pub use ::async_trait::async_trait;

/// Message relay trait for MPC communication
#[async_trait]
pub trait Relay: Send + Sync {
    /// Broadcast a message to all parties
    async fn broadcast<T: Serialize + Send + Sync>(
        &self,
        session_id: &SessionId,
        round: u32,
        message: &T,
    ) -> Result<()>;

    /// Send a direct message to a specific party
    async fn send_direct<T: Serialize + Send + Sync>(
        &self,
        session_id: &SessionId,
        round: u32,
        to: PartyId,
        message: &T,
    ) -> Result<()>;

    /// Collect broadcast messages from all parties
    async fn collect_broadcasts<T: DeserializeOwned + Send>(
        &self,
        session_id: &SessionId,
        round: u32,
        count: usize,
    ) -> Result<Vec<T>>;

    /// Collect direct messages sent to this party
    async fn collect_direct<T: DeserializeOwned + Send>(
        &self,
        session_id: &SessionId,
        round: u32,
        my_id: PartyId,
        count: usize,
    ) -> Result<Vec<T>>;
}

/// In-memory relay for testing
pub mod memory;

pub use memory::MemoryRelay;
