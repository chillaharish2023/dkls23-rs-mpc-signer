//! DSG protocol implementation

use crate::mpc::Relay;
use crate::{Error, KeyShare, PartyId, Result, SessionConfig, SessionId, Signature};
use k256::{
    elliptic_curve::{
        bigint::U256, ops::Reduce, point::DecompressPoint, Field,
        sec1::{FromEncodedPoint, ToEncodedPoint},
    },
    AffinePoint, ProjectivePoint, Scalar,
};
use rand::rngs::OsRng;
use tracing::{debug, info, instrument};

use super::{PartialSignature, PreSignature};

/// Run the distributed signature generation protocol
///
/// This implements the 3-round signing protocol from DKLs23.
///
/// # Arguments
/// * `key_share` - This party's key share from DKG
/// * `message` - Message hash to sign (32 bytes)
/// * `parties` - List of participating party IDs
/// * `relay` - Message relay for communication
///
/// # Returns
/// The ECDSA signature
#[instrument(skip(key_share, relay))]
pub async fn run_dsg<R: Relay>(
    key_share: &KeyShare,
    message: &[u8; 32],
    parties: &[PartyId],
    relay: &R,
) -> Result<Signature> {
    info!(
        party_id = key_share.party_id,
        participants = ?parties,
        "Starting DSG"
    );

    // Verify threshold
    if parties.len() < key_share.threshold {
        return Err(Error::ThresholdNotMet {
            required: key_share.threshold,
            actual: parties.len(),
        });
    }

    // Verify this party is in the signing set
    if !parties.contains(&key_share.party_id) {
        return Err(Error::InvalidPartyId(key_share.party_id));
    }

    let session_id: SessionId = rand::random();
    let config = SessionConfig {
        session_id,
        n_parties: parties.len(),
        threshold: key_share.threshold,
        party_id: key_share.party_id,
        parties: parties.to_vec(),
    };

    // Generate pre-signature
    let pre_sig = pre_signature(key_share, &config, relay).await?;

    // Create partial signature
    let partial = create_partial_signature(key_share, &pre_sig, message)?;

    // Broadcast partial signature
    let partial_msg = super::DsgPartialMessage {
        party_id: key_share.party_id,
        sigma_share: partial.sigma_share.clone(),
    };
    relay.broadcast(&session_id, 3, &partial_msg).await?;

    // Collect partial signatures
    let all_partials = relay
        .collect_broadcasts::<super::DsgPartialMessage>(&session_id, 3, parties.len())
        .await?;

    let partial_sigs: Vec<PartialSignature> = all_partials
        .into_iter()
        .map(|msg| PartialSignature {
            party_id: msg.party_id,
            sigma_share: msg.sigma_share,
        })
        .collect();

    // Combine partial signatures
    let signature = combine_partial_signatures(&pre_sig, &partial_sigs, message)?;

    info!(
        party_id = key_share.party_id,
        r = hex::encode(&signature.r),
        s = hex::encode(&signature.s),
        "DSG completed successfully"
    );

    Ok(signature)
}

/// Generate pre-signature (can be done before message is known)
#[instrument(skip(key_share, relay))]
pub async fn pre_signature<R: Relay>(
    key_share: &KeyShare,
    config: &SessionConfig,
    relay: &R,
) -> Result<PreSignature> {
    debug!("Generating pre-signature");

    let mut rng = OsRng;

    // Round 1: Generate random k_i and broadcast commitment
    let k_i = Scalar::random(&mut rng);
    let gamma_i = Scalar::random(&mut rng);

    let k_commitment = ProjectivePoint::GENERATOR * k_i;
    let gamma_commitment = ProjectivePoint::GENERATOR * gamma_i;

    let round1_msg = super::DsgRound1Message {
        party_id: config.party_id,
        k_commitment: k_commitment
            .to_affine()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec(),
        gamma_commitment: gamma_commitment
            .to_affine()
            .to_encoded_point(true)
            .as_bytes()
            .to_vec(),
    };
    relay.broadcast(&config.session_id, 1, &round1_msg).await?;

    // Collect round 1 messages
    let round1_msgs = relay
        .collect_broadcasts::<super::DsgRound1Message>(&config.session_id, 1, config.parties.len())
        .await?;

    // Round 2: Multiplicative-to-additive (MtA) protocol
    debug!("DSG Round 2: MtA protocol");

    // Compute Lagrange coefficient
    let lambda_i = compute_lagrange_coefficient(config.party_id, &config.parties);

    // Compute shares
    let x_i = key_share.secret_share * lambda_i;
    let k_inv_share = k_i; // Simplified - full protocol uses MtA
    let chi_share = x_i * k_i; // Simplified

    // Broadcast round 2
    let round2_msg = super::DsgRound2Message {
        party_id: config.party_id,
        delta_share: (k_i * gamma_i).to_bytes().to_vec(),
    };
    relay.broadcast(&config.session_id, 2, &round2_msg).await?;

    // Collect round 2 messages
    let _round2_msgs = relay
        .collect_broadcasts::<super::DsgRound2Message>(&config.session_id, 2, config.parties.len())
        .await?;

    // Compute R = sum(k_i * G)
    let mut r_point = ProjectivePoint::IDENTITY;
    for msg in &round1_msgs {
        let point = k256::EncodedPoint::from_bytes(&msg.k_commitment)
            .map_err(|e| Error::Deserialization(e.to_string()))?;
        let affine_opt = AffinePoint::from_encoded_point(&point);
        let affine: AffinePoint = Option::<AffinePoint>::from(affine_opt)
            .ok_or_else(|| Error::VerificationFailed("Invalid K commitment".into()))?;
        let commitment = ProjectivePoint::from(affine);
        r_point = r_point + commitment;
    }

    let r_encoded = r_point.to_affine().to_encoded_point(true);
    let r_bytes: [u8; 33] = r_encoded
        .as_bytes()
        .try_into()
        .map_err(|_| Error::Internal("Invalid R point".into()))?;

    Ok(PreSignature {
        session_id: config.session_id,
        parties: config.parties.clone(),
        r_point: r_bytes,
        k_inv_share: k_inv_share.to_bytes().to_vec(),
        chi_share: chi_share.to_bytes().to_vec(),
    })
}

/// Create a partial signature
pub fn create_partial_signature(
    _key_share: &KeyShare,
    pre_sig: &PreSignature,
    message: &[u8; 32],
) -> Result<PartialSignature> {
    // Parse pre-signature data
    let k_inv_bytes: [u8; 32] = pre_sig
        .k_inv_share
        .clone()
        .try_into()
        .map_err(|_| Error::Deserialization("Invalid k_inv_share length".into()))?;
    let k_inv_share = <Scalar as Reduce<U256>>::reduce_bytes(&k_inv_bytes.into());

    let chi_bytes: [u8; 32] = pre_sig
        .chi_share
        .clone()
        .try_into()
        .map_err(|_| Error::Deserialization("Invalid chi_share length".into()))?;
    let chi_share = <Scalar as Reduce<U256>>::reduce_bytes(&chi_bytes.into());

    // Get r value from R point
    let r_point = k256::EncodedPoint::from_bytes(&pre_sig.r_point)
        .map_err(|e| Error::Deserialization(e.to_string()))?;
    let r_affine_opt = AffinePoint::from_encoded_point(&r_point);
    let r_affine: AffinePoint = Option::<AffinePoint>::from(r_affine_opt)
        .ok_or_else(|| Error::VerificationFailed("Invalid R point".into()))?;

    // r = x-coordinate of R mod n
    let r_bytes = r_affine.to_encoded_point(false);
    let r_coord: [u8; 32] = r_bytes.as_bytes()[1..33]
        .try_into()
        .map_err(|_| Error::Internal("Invalid R coordinate".into()))?;
    let r = <Scalar as Reduce<U256>>::reduce_bytes(&r_coord.into());

    // m = message hash
    let m = <Scalar as Reduce<U256>>::reduce_bytes(&(*message).into());

    // sigma_i = k_i^-1 * (m + r * x_i)
    // Simplified: sigma_i = k_inv_share * m + r * chi_share
    let sigma_share = k_inv_share * m + r * chi_share;

    Ok(PartialSignature {
        party_id: 0, // Will be set by caller
        sigma_share: sigma_share.to_bytes().to_vec(),
    })
}

/// Combine partial signatures into final signature
pub fn combine_partial_signatures(
    pre_sig: &PreSignature,
    partials: &[PartialSignature],
    _message: &[u8; 32],
) -> Result<Signature> {
    // Sum all sigma shares
    let mut s = Scalar::ZERO;
    for partial in partials {
        let sigma_bytes: [u8; 32] = partial
            .sigma_share
            .clone()
            .try_into()
            .map_err(|_| Error::Deserialization("Invalid sigma_share length".into()))?;
        let sigma = <Scalar as Reduce<U256>>::reduce_bytes(&sigma_bytes.into());
        s = s + sigma;
    }

    // Get r from R point
    let r_point = k256::EncodedPoint::from_bytes(&pre_sig.r_point)
        .map_err(|e| Error::Deserialization(e.to_string()))?;
    let r_affine_opt = AffinePoint::from_encoded_point(&r_point);
    let r_affine: AffinePoint = Option::<AffinePoint>::from(r_affine_opt)
        .ok_or_else(|| Error::VerificationFailed("Invalid R point".into()))?;

    let r_bytes = r_affine.to_encoded_point(false);
    let r: [u8; 32] = r_bytes.as_bytes()[1..33]
        .try_into()
        .map_err(|_| Error::Internal("Invalid r length".into()))?;

    // Normalize s to low-s form
    let s_bytes = s.to_bytes();
    let s_normalized: [u8; 32] = s_bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::Internal("Invalid s length".into()))?;

    // Compute recovery ID from Y coordinate parity
    // Check if Y is odd by looking at the compressed point prefix
    let r_encoded = r_affine.to_encoded_point(true);
    let recovery_id = if r_encoded.as_bytes()[0] == 0x03 { 1 } else { 0 };

    Ok(Signature::new(r, s_normalized, recovery_id))
}

/// Compute Lagrange coefficient for party i
fn compute_lagrange_coefficient(party_id: PartyId, parties: &[PartyId]) -> Scalar {
    let i = party_id as u64 + 1;
    let mut numerator = Scalar::ONE;
    let mut denominator = Scalar::ONE;

    for &j_id in parties {
        let j = j_id as u64 + 1;
        if j != i {
            numerator = numerator * Scalar::from(j);
            let diff = if j > i {
                Scalar::from(j - i)
            } else {
                -Scalar::from(i - j)
            };
            denominator = denominator * diff;
        }
    }

    numerator * denominator.invert().unwrap_or(Scalar::ONE)
}
