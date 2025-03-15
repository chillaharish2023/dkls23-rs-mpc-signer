//! Message Relay Library
//!
//! Provides the core message relay functionality for MPC communication.
//! Supports both broadcast and point-to-point messaging with message caching
//! for temporarily offline devices.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Relay error types
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("Message not found: {0}")]
    NotFound(String),
    #[error("Invalid message format: {0}")]
    InvalidFormat(String),
    #[error("Session expired: {0}")]
    SessionExpired(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, RelayError>;

/// Message identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MessageId {
    /// Session identifier
    pub session_id: String,
    /// Round number
    pub round: u32,
    /// Sender party ID (None for broadcasts)
    pub from: Option<usize>,
    /// Receiver party ID (None for broadcasts)
    pub to: Option<usize>,
    /// Message tag
    pub tag: String,
}

impl MessageId {
    /// Create a new message ID
    pub fn new(session_id: &str, round: u32, from: Option<usize>, to: Option<usize>, tag: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            round,
            from,
            to,
            tag: tag.to_string(),
        }
    }

    /// Compute hash for lookup
    pub fn hash(&self) -> String {
        let data = format!(
            "{}:{}:{}:{}:{}",
            self.session_id,
            self.round,
            self.from.map(|v| v.to_string()).unwrap_or_default(),
            self.to.map(|v| v.to_string()).unwrap_or_default(),
            self.tag
        );
        hex::encode(blake3::hash(data.as_bytes()).as_bytes())
    }
}

/// Stored message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    /// Message ID
    pub id: MessageId,
    /// Message payload
    pub payload: Vec<u8>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
}

/// Message relay store
#[derive(Clone)]
pub struct MessageStore {
    /// Messages indexed by hash
    messages: Arc<DashMap<String, StoredMessage>>,
    /// Default TTL in seconds
    ttl_seconds: i64,
}

impl MessageStore {
    /// Create a new message store
    pub fn new(ttl_seconds: i64) -> Self {
        Self {
            messages: Arc::new(DashMap::new()),
            ttl_seconds,
        }
    }

    /// Store a message
    pub fn put(&self, id: MessageId, payload: Vec<u8>) -> Result<()> {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::seconds(self.ttl_seconds);

        let message = StoredMessage {
            id: id.clone(),
            payload,
            created_at: now,
            expires_at,
        };

        self.messages.insert(id.hash(), message);
        Ok(())
    }

    /// Get a message by ID
    pub fn get(&self, id: &MessageId) -> Result<StoredMessage> {
        let hash = id.hash();

        self.messages
            .get(&hash)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| RelayError::NotFound(hash))
    }

    /// Check if a message exists
    pub fn exists(&self, id: &MessageId) -> bool {
        self.messages.contains_key(&id.hash())
    }

    /// Remove expired messages
    pub fn cleanup(&self) {
        let now = Utc::now();
        self.messages.retain(|_, v| v.expires_at > now);
    }

    /// Get all messages for a session and round
    pub fn get_round_messages(&self, session_id: &str, round: u32) -> Vec<StoredMessage> {
        self.messages
            .iter()
            .filter(|entry| {
                entry.id.session_id == session_id && entry.id.round == round
            })
            .map(|entry| entry.value().clone())
            .collect()
    }
}

impl Default for MessageStore {
    fn default() -> Self {
        Self::new(3600) // 1 hour default TTL
    }
}

/// Peer relay connection info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer URL
    pub url: String,
    /// Is this peer active
    pub active: bool,
    /// Last seen timestamp
    pub last_seen: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_id_hash() {
        let id1 = MessageId::new("session1", 1, Some(0), Some(1), "keygen");
        let id2 = MessageId::new("session1", 1, Some(0), Some(1), "keygen");
        let id3 = MessageId::new("session1", 2, Some(0), Some(1), "keygen");

        assert_eq!(id1.hash(), id2.hash());
        assert_ne!(id1.hash(), id3.hash());
    }

    #[test]
    fn test_message_store() {
        let store = MessageStore::new(3600);
        let id = MessageId::new("session1", 1, Some(0), None, "broadcast");

        store.put(id.clone(), vec![1, 2, 3]).unwrap();

        assert!(store.exists(&id));

        let msg = store.get(&id).unwrap();
        assert_eq!(msg.payload, vec![1, 2, 3]);
    }
}
