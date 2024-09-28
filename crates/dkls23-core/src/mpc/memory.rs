//! In-memory relay implementation for testing

use super::{async_trait, Relay};
use crate::{Error, PartyId, Result, SessionId};
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// In-memory message relay for local testing
pub struct MemoryRelay {
    /// Broadcast messages: (session_id, round) -> Vec<message_bytes>
    broadcasts: Arc<DashMap<(SessionId, u32), Vec<Vec<u8>>>>,
    /// Direct messages: (session_id, round, to) -> Vec<message_bytes>
    directs: Arc<DashMap<(SessionId, u32, PartyId), Vec<Vec<u8>>>>,
    /// Notification channel
    notify: broadcast::Sender<()>,
}

impl MemoryRelay {
    /// Create a new in-memory relay
    pub fn new() -> Self {
        let (notify, _) = broadcast::channel(100);
        Self {
            broadcasts: Arc::new(DashMap::new()),
            directs: Arc::new(DashMap::new()),
            notify,
        }
    }
}

impl Default for MemoryRelay {
    fn default() -> Self {
        Self::new()
    }
}

fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| Error::Serialization(e.to_string()))
}

fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| Error::Deserialization(e.to_string()))
}

#[async_trait]
impl Relay for MemoryRelay {
    async fn broadcast<T: Serialize + Send + Sync>(
        &self,
        session_id: &SessionId,
        round: u32,
        message: &T,
    ) -> Result<()> {
        let bytes = serialize(message)?;

        self.broadcasts
            .entry((*session_id, round))
            .or_default()
            .push(bytes);

        let _ = self.notify.send(());
        Ok(())
    }

    async fn send_direct<T: Serialize + Send + Sync>(
        &self,
        session_id: &SessionId,
        round: u32,
        to: PartyId,
        message: &T,
    ) -> Result<()> {
        let bytes = serialize(message)?;

        self.directs
            .entry((*session_id, round, to))
            .or_default()
            .push(bytes);

        let _ = self.notify.send(());
        Ok(())
    }

    async fn collect_broadcasts<T: DeserializeOwned + Send>(
        &self,
        session_id: &SessionId,
        round: u32,
        count: usize,
    ) -> Result<Vec<T>> {
        let mut rx = self.notify.subscribe();

        loop {
            if let Some(messages) = self.broadcasts.get(&(*session_id, round)) {
                if messages.len() >= count {
                    let result: Result<Vec<T>> = messages
                        .iter()
                        .take(count)
                        .map(|bytes| deserialize(bytes))
                        .collect();
                    return result;
                }
            }

            // Wait for notification with timeout
            tokio::select! {
                _ = rx.recv() => continue,
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => continue,
            }
        }
    }

    async fn collect_direct<T: DeserializeOwned + Send>(
        &self,
        session_id: &SessionId,
        round: u32,
        my_id: PartyId,
        count: usize,
    ) -> Result<Vec<T>> {
        let mut rx = self.notify.subscribe();

        loop {
            if let Some(messages) = self.directs.get(&(*session_id, round, my_id)) {
                if messages.len() >= count {
                    let result: Result<Vec<T>> = messages
                        .iter()
                        .take(count)
                        .map(|bytes| deserialize(bytes))
                        .collect();
                    return result;
                }
            }

            // Wait for notification with timeout
            tokio::select! {
                _ = rx.recv() => continue,
                _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => continue,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        value: u32,
    }

    #[tokio::test]
    async fn test_broadcast() {
        let relay = MemoryRelay::new();
        let session_id = [0u8; 32];

        relay.broadcast(&session_id, 1, &TestMessage { value: 42 }).await.unwrap();
        relay.broadcast(&session_id, 1, &TestMessage { value: 43 }).await.unwrap();

        let messages: Vec<TestMessage> = relay.collect_broadcasts(&session_id, 1, 2).await.unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].value, 42);
        assert_eq!(messages[1].value, 43);
    }

    #[tokio::test]
    async fn test_direct() {
        let relay = MemoryRelay::new();
        let session_id = [0u8; 32];

        relay.send_direct(&session_id, 1, 0, &TestMessage { value: 100 }).await.unwrap();

        let messages: Vec<TestMessage> = relay.collect_direct(&session_id, 1, 0, 1).await.unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].value, 100);
    }
}
