//! Core types for DKLs23 protocol

use k256::{
    ecdsa,
    elliptic_curve::{bigint::U256, ops::Reduce, sec1::FromEncodedPoint},
    AffinePoint, ProjectivePoint, Scalar,
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Unique identifier for a party in the MPC network
pub type PartyId = usize;

/// Unique identifier for a session
pub type SessionId = [u8; 32];

/// Compressed public key bytes
pub type PublicKey = [u8; 33];

/// ECDSA signature (r, s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// R component
    pub r: [u8; 32],
    /// S component
    pub s: [u8; 32],
    /// Recovery ID (0 or 1)
    pub recovery_id: u8,
}

impl Signature {
    /// Create a new signature
    pub fn new(r: [u8; 32], s: [u8; 32], recovery_id: u8) -> Self {
        Self { r, s, recovery_id }
    }

    /// Convert to DER format
    pub fn to_der(&self) -> Vec<u8> {
        let sig = ecdsa::Signature::from_scalars(
            *k256::FieldBytes::from_slice(&self.r),
            *k256::FieldBytes::from_slice(&self.s),
        )
        .expect("valid signature");
        sig.to_der().as_bytes().to_vec()
    }

    /// Convert to bytes (r || s)
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&self.r);
        bytes[32..].copy_from_slice(&self.s);
        bytes
    }
}

/// Wrapper for Scalar serialization
#[derive(Clone)]
pub struct ScalarWrapper(pub Scalar);

impl Serialize for ScalarWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.0.to_bytes();
        serializer.serialize_bytes(bytes.as_slice())
    }
}

impl<'de> Deserialize<'de> for ScalarWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let array: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("Invalid scalar length"))?;
        Ok(ScalarWrapper(<Scalar as Reduce<U256>>::reduce_bytes(
            &array.into(),
        )))
    }
}

impl Zeroize for ScalarWrapper {
    fn zeroize(&mut self) {
        // Scalar doesn't implement Zeroize directly, but we can't zeroize it here
        // In a production system, you'd want to handle this more carefully
    }
}

/// Key share held by a party after DKG
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct KeyShare {
    /// This party's ID
    pub party_id: PartyId,

    /// Total number of parties
    pub n_parties: usize,

    /// Threshold
    pub threshold: usize,

    /// This party's secret share (x_i) - stored as bytes for serialization
    #[zeroize(skip)]
    #[serde(with = "scalar_serde")]
    pub secret_share: Scalar,

    /// Public key (compressed) - stored as Vec for serde compatibility
    #[zeroize(skip)]
    pub public_key: Vec<u8>,

    /// Public key shares of all parties
    #[zeroize(skip)]
    pub public_shares: Vec<Vec<u8>>,

    /// Chain code for BIP32 derivation
    pub chain_code: [u8; 32],
}

mod scalar_serde {
    use k256::{elliptic_curve::{bigint::U256, ops::Reduce}, Scalar};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(scalar: &Scalar, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = scalar.to_bytes();
        serializer.serialize_bytes(bytes.as_slice())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Scalar, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = Vec::deserialize(deserializer)?;
        let array: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("Invalid scalar length"))?;
        Ok(<Scalar as Reduce<U256>>::reduce_bytes(&array.into()))
    }
}

impl KeyShare {
    /// Get the public key as a ProjectivePoint
    pub fn public_key_point(&self) -> ProjectivePoint {
        let encoded = k256::EncodedPoint::from_bytes(&self.public_key).expect("valid point");
        let affine_opt = AffinePoint::from_encoded_point(&encoded);
        let affine: AffinePoint = Option::<AffinePoint>::from(affine_opt).expect("valid point");
        ProjectivePoint::from(affine)
    }

    /// Derive a child key share using non-hardened BIP32 derivation
    pub fn derive_child(&self, path: &str) -> crate::Result<KeyShare> {
        use derivation_path::DerivationPath;

        let derivation_path: DerivationPath = path
            .parse()
            .map_err(|e| crate::Error::Derivation(format!("Invalid path: {}", e)))?;

        let mut current_share = self.clone();
        let mut current_chain_code = self.chain_code;

        // Get path components
        let components: Vec<_> = derivation_path.into_iter().collect();
        
        for child_index in components {
            if child_index.is_hardened() {
                return Err(crate::Error::Derivation(
                    "Hardened derivation not supported in threshold setting".into(),
                ));
            }

            // Extract the index value
            let index = match child_index {
                derivation_path::ChildIndex::Normal(idx) => *idx,
                derivation_path::ChildIndex::Hardened(_) => {
                    return Err(crate::Error::Derivation(
                        "Hardened derivation not supported".into(),
                    ));
                }
            };

            let (new_share, new_chain_code) =
                derive_non_hardened(&current_share, current_chain_code, index)?;

            current_share.secret_share = new_share;
            current_chain_code = new_chain_code;
        }

        current_share.chain_code = current_chain_code;
        Ok(current_share)
    }
}

/// Derive non-hardened child key
fn derive_non_hardened(
    parent: &KeyShare,
    chain_code: [u8; 32],
    index: u32,
) -> crate::Result<(Scalar, [u8; 32])> {
    use hmac::{Hmac, Mac};
    use sha2::Sha512;

    // Compute HMAC-SHA512(chain_code, public_key || index)
    let mut hmac = Hmac::<Sha512>::new_from_slice(&chain_code)
        .map_err(|e| crate::Error::Derivation(e.to_string()))?;

    // Use compressed public key
    hmac.update(&parent.public_key);
    hmac.update(&index.to_be_bytes());

    let result = hmac.finalize().into_bytes();

    // Split into secret addition and new chain code
    let secret_bytes: [u8; 32] = result[..32].try_into().unwrap();
    let secret_add = <Scalar as Reduce<U256>>::reduce_bytes(&secret_bytes.into());
    let new_chain_code: [u8; 32] = result[32..].try_into().unwrap();

    // Add to parent secret share
    let new_secret = parent.secret_share + secret_add;

    Ok((new_secret, new_chain_code))
}

/// Configuration for DKG/DSG sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session identifier
    pub session_id: SessionId,

    /// Number of parties
    pub n_parties: usize,

    /// Threshold (t-of-n)
    pub threshold: usize,

    /// This party's ID
    pub party_id: PartyId,

    /// List of participating party IDs
    pub parties: Vec<PartyId>,
}

impl SessionConfig {
    /// Create a new session configuration
    pub fn new(n_parties: usize, threshold: usize, party_id: PartyId) -> crate::Result<Self> {
        if threshold > n_parties {
            return Err(crate::Error::InvalidConfig(
                "Threshold cannot exceed number of parties".into(),
            ));
        }
        if threshold < 2 {
            return Err(crate::Error::InvalidConfig(
                "Threshold must be at least 2".into(),
            ));
        }

        let session_id = rand::random();
        let parties = (0..n_parties).collect();

        Ok(Self {
            session_id,
            n_parties,
            threshold,
            party_id,
            parties,
        })
    }
}

/// Message types exchanged during protocol execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Broadcast message to all parties
    Broadcast {
        from: PartyId,
        round: u32,
        data: Vec<u8>,
    },
    /// Point-to-point message
    Direct {
        from: PartyId,
        to: PartyId,
        round: u32,
        data: Vec<u8>,
    },
}

impl Message {
    /// Get the sender of this message
    pub fn sender(&self) -> PartyId {
        match self {
            Message::Broadcast { from, .. } => *from,
            Message::Direct { from, .. } => *from,
        }
    }

    /// Get the round number
    pub fn round(&self) -> u32 {
        match self {
            Message::Broadcast { round, .. } => *round,
            Message::Direct { round, .. } => *round,
        }
    }
}
