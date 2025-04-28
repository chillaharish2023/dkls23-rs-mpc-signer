//! Message Relay Client
//!
//! Client library for communicating with the message relay service.

use dkls23_core::mpc::{async_trait, Relay};
use dkls23_core::{Error, PartyId, Result, SessionId};
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, instrument};

/// HTTP-based relay client
pub struct RelayClient {
    /// HTTP client
    client: Client,
    /// Relay service URL
    url: String,
    /// This party's ID
    party_id: PartyId,
    /// Request timeout
    timeout: Duration,
}

impl RelayClient {
    /// Create a new relay client
    pub fn new(url: &str, party_id: PartyId) -> Self {
        Self {
            client: Client::new(),
            url: url.trim_end_matches('/').to_string(),
            party_id,
            timeout: Duration::from_secs(30),
        }
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Post a message to the relay
    #[instrument(skip(self, payload))]
    async fn post_message(
        &self,
        session_id: &SessionId,
        round: u32,
        to: Option<PartyId>,
        tag: &str,
        payload: &[u8],
    ) -> Result<()> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        
        let req = PostMessageRequest {
            session_id: hex::encode(session_id),
            round,
            from: Some(self.party_id),
            to,
            tag: tag.to_string(),
            payload: STANDARD.encode(payload),
        };

        let response = self
            .client
            .post(format!("{}/v1/msg", self.url))
            .json(&req)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Relay(format!(
                "POST failed with status: {}",
                response.status()
            )));
        }

        debug!(round, to = ?to, "Message posted");
        Ok(())
    }

    /// Get a message from the relay
    #[instrument(skip(self))]
    async fn get_message(
        &self,
        session_id: &SessionId,
        round: u32,
        from: Option<PartyId>,
        to: Option<PartyId>,
        tag: &str,
    ) -> Result<Option<Vec<u8>>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        
        let req = GetMessageRequest {
            session_id: hex::encode(session_id),
            round,
            from,
            to,
            tag: tag.to_string(),
        };

        let response = self
            .client
            .get(format!("{}/v1/msg", self.url))
            .json(&req)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Relay(format!(
                "GET failed with status: {}",
                response.status()
            )));
        }

        let msg_response: MessageResponse = response
            .json()
            .await
            .map_err(|e| Error::Relay(e.to_string()))?;

        if msg_response.found {
            let payload = STANDARD.decode(&msg_response.payload.unwrap_or_default())
                .map_err(|e| Error::Deserialization(e.to_string()))?;
            Ok(Some(payload))
        } else {
            Ok(None)
        }
    }
}

fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| Error::Serialization(e.to_string()))
}

fn deserialize<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| Error::Deserialization(e.to_string()))
}

#[async_trait]
impl Relay for RelayClient {
    async fn broadcast<T: Serialize + Send + Sync>(
        &self,
        session_id: &SessionId,
        round: u32,
        message: &T,
    ) -> Result<()> {
        let payload = serialize(message)?;
        self.post_message(session_id, round, None, "broadcast", &payload)
            .await
    }

    async fn send_direct<T: Serialize + Send + Sync>(
        &self,
        session_id: &SessionId,
        round: u32,
        to: PartyId,
        message: &T,
    ) -> Result<()> {
        let payload = serialize(message)?;
        self.post_message(session_id, round, Some(to), "direct", &payload)
            .await
    }

    async fn collect_broadcasts<T: DeserializeOwned + Send>(
        &self,
        session_id: &SessionId,
        round: u32,
        count: usize,
    ) -> Result<Vec<T>> {
        let mut messages = Vec::new();
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 100;

        while messages.len() < count && attempts < MAX_ATTEMPTS {
            for party_id in 0..count {
                if let Some(payload) = self
                    .get_message(session_id, round, Some(party_id), None, "broadcast")
                    .await?
                {
                    let msg: T = deserialize(&payload)?;
                    messages.push(msg);
                }
            }

            if messages.len() < count {
                tokio::time::sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
        }

        if messages.len() < count {
            return Err(Error::Timeout(format!(
                "Waiting for {} broadcast messages in round {}",
                count, round
            )));
        }

        Ok(messages)
    }

    async fn collect_direct<T: DeserializeOwned + Send>(
        &self,
        session_id: &SessionId,
        round: u32,
        my_id: PartyId,
        count: usize,
    ) -> Result<Vec<T>> {
        let mut messages = Vec::new();
        let mut attempts = 0;
        const MAX_ATTEMPTS: usize = 100;

        while messages.len() < count && attempts < MAX_ATTEMPTS {
            // Try to get messages from each possible sender
            for sender in 0..count + 1 {
                if sender == my_id {
                    continue;
                }
                if let Some(payload) = self
                    .get_message(session_id, round, Some(sender), Some(my_id), "direct")
                    .await?
                {
                    let msg: T = deserialize(&payload)?;
                    messages.push(msg);
                }
            }

            if messages.len() < count {
                tokio::time::sleep(Duration::from_millis(100)).await;
                attempts += 1;
            }
        }

        if messages.len() < count {
            return Err(Error::Timeout(format!(
                "Waiting for {} direct messages in round {}",
                count, round
            )));
        }

        Ok(messages)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PostMessageRequest {
    session_id: String,
    round: u32,
    from: Option<usize>,
    to: Option<usize>,
    tag: String,
    payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetMessageRequest {
    session_id: String,
    round: u32,
    from: Option<usize>,
    to: Option<usize>,
    tag: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MessageResponse {
    found: bool,
    payload: Option<String>,
}
