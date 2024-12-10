//! DSG message types

use crate::PartyId;
use serde::{Deserialize, Serialize};

/// Round 1 message: Commitments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsgRound1Message {
    /// Sender party ID
    pub party_id: PartyId,
    /// Commitment to k_i
    pub k_commitment: Vec<u8>,
    /// Commitment to gamma_i
    pub gamma_commitment: Vec<u8>,
}

/// Round 2 message: MtA protocol data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsgRound2Message {
    /// Sender party ID
    pub party_id: PartyId,
    /// Delta share
    pub delta_share: Vec<u8>,
}

/// Round 3 message: Partial signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DsgPartialMessage {
    /// Sender party ID
    pub party_id: PartyId,
    /// Sigma share
    pub sigma_share: Vec<u8>,
}
