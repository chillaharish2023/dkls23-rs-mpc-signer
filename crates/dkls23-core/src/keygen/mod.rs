//! Distributed Key Generation (DKG) module
//!
//! Implements the DKG protocol from DKLs23 for generating threshold ECDSA keys.

mod dkg;
mod key_refresh;
mod messages;

pub use dkg::run_dkg;
pub use key_refresh::run_key_refresh;
pub use messages::*;

use crate::{KeyShare, Result, SessionConfig};

/// DKG state machine
pub struct DkgSession {
    config: SessionConfig,
    round: u32,
    commitments: Vec<Vec<u8>>,
    shares: Vec<Vec<u8>>,
}

impl DkgSession {
    /// Create a new DKG session
    pub fn new(config: SessionConfig) -> Self {
        Self {
            config,
            round: 0,
            commitments: Vec::new(),
            shares: Vec::new(),
        }
    }

    /// Get current round
    pub fn round(&self) -> u32 {
        self.round
    }

    /// Check if DKG is complete
    pub fn is_complete(&self) -> bool {
        self.round >= 3
    }
}
