//! # DKLs23 Core
//!
//! Core cryptographic primitives for DKLs23 threshold ECDSA protocol.
//!
//! This crate provides the fundamental building blocks for:
//! - Distributed Key Generation (DKG)
//! - Key Refresh
//! - Distributed Signature Generation (DSG)
//!
//! ## Protocol Overview
//!
//! DKLs23 is a three-round threshold ECDSA protocol that achieves:
//! - UC security (Universally Composable)
//! - No explicit ZK proofs during signing
//! - Black-box use of 2-round 2P-MUL
//!
//! ## Example
//!
//! ```rust,ignore
//! use dkls23_core::{keygen, sign, KeyShare};
//!
//! // Run distributed key generation
//! let key_share = keygen::run_dkg(&config, &relay).await?;
//!
//! // Sign a message
//! let signature = sign::run_dsg(&key_share, message, &relay).await?;
//! ```

pub mod error;
pub mod keygen;
pub mod mpc;
pub mod oblivious;
pub mod sign;
pub mod types;

pub use error::{Error, Result};
pub use types::{KeyShare, PartyId, PublicKey, SessionConfig, SessionId, Signature};

/// Protocol version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default threshold for a 3-party setup
pub const DEFAULT_THRESHOLD: usize = 2;

/// Default number of parties
pub const DEFAULT_PARTIES: usize = 3;
