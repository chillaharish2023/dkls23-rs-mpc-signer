//! Error types for DKLs23 operations

use thiserror::Error;

/// Result type alias for DKLs23 operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during DKLs23 protocol execution
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid party configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Threshold requirements not met
    #[error("Threshold not met: required {required}, got {actual}")]
    ThresholdNotMet { required: usize, actual: usize },

    /// Invalid party ID
    #[error("Invalid party ID: {0}")]
    InvalidPartyId(usize),

    /// Message verification failed
    #[error("Message verification failed: {0}")]
    VerificationFailed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Cryptographic operation failed
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// Network/relay error
    #[error("Relay error: {0}")]
    Relay(String),

    /// Timeout waiting for message
    #[error("Timeout waiting for {0}")]
    Timeout(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Invalid signature
    #[error("Invalid signature")]
    InvalidSignature,

    /// Key derivation error
    #[error("Key derivation error: {0}")]
    Derivation(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}
