//! Distributed Signature Generation (DSG) module
//!
//! Implements the three-round signing protocol from DKLs23.

mod dsg;
mod messages;

pub use dsg::{create_partial_signature, pre_signature, run_dsg, combine_partial_signatures};
pub use messages::*;

use crate::{KeyShare, PartyId, Result, SessionId, Signature};

/// Pre-signature data (before message hash is known)
#[derive(Clone)]
pub struct PreSignature {
    /// Session ID
    pub session_id: SessionId,
    /// Participating parties
    pub parties: Vec<PartyId>,
    /// R point (compressed)
    pub r_point: [u8; 33],
    /// Party's share of k^-1
    pub k_inv_share: Vec<u8>,
    /// Party's multiplicative share
    pub chi_share: Vec<u8>,
}

/// Partial signature from one party
#[derive(Clone)]
pub struct PartialSignature {
    /// Party ID
    pub party_id: PartyId,
    /// Sigma share
    pub sigma_share: Vec<u8>,
}
