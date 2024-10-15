//! DKG message types

use crate::PartyId;
use serde::{Deserialize, Serialize};

/// Round 1 message: Commitment to secret polynomial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound1Message {
    /// Sender party ID
    pub party_id: PartyId,
    /// Commitments to polynomial coefficients (Feldman VSS)
    pub commitments: Vec<Vec<u8>>,
}

/// Round 2 message: Secret share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound2Message {
    /// Sender party ID
    pub from: PartyId,
    /// Receiver party ID
    pub to: PartyId,
    /// Encrypted secret share
    pub share: Vec<u8>,
}

/// Round 3 message: Completion acknowledgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkgRound3Message {
    /// Sender party ID
    pub party_id: PartyId,
    /// Public key share verification
    pub public_share: Vec<u8>,
}
