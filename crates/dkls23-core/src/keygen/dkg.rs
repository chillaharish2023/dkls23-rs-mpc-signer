//! DKG protocol implementation

use crate::mpc::Relay;
use crate::{Error, KeyShare, Result, SessionConfig};
use k256::{
    elliptic_curve::{
        bigint::U256, group::GroupEncoding, ops::Reduce, subtle::CtOption, Field, Group,
        sec1::{FromEncodedPoint, ToEncodedPoint},
    },
    AffinePoint, ProjectivePoint, Scalar,
};
use rand::rngs::OsRng;
use tracing::{debug, info, instrument};

/// Run the distributed key generation protocol
///
/// This implements the DKG from Protocol 6.1 of the DKLs23 paper.
///
/// # Arguments
/// * `config` - Session configuration
/// * `relay` - Message relay for communication
///
/// # Returns
/// The party's key share after successful DKG
#[instrument(skip(relay))]
pub async fn run_dkg<R: Relay>(config: &SessionConfig, relay: &R) -> Result<KeyShare> {
    info!(
        party_id = config.party_id,
        n_parties = config.n_parties,
        threshold = config.threshold,
        "Starting DKG"
    );

    // Round 1: Generate and commit to secret polynomial
    debug!("DKG Round 1: Commitment");
    let (secret_poly, commitments) = generate_secret_polynomial(config)?;

    // Broadcast commitment
    let commitment_msg = super::DkgRound1Message {
        party_id: config.party_id,
        commitments: commitments.clone(),
    };
    relay
        .broadcast(&config.session_id, 1, &commitment_msg)
        .await?;

    // Collect commitments from all parties
    let all_commitments = relay
        .collect_broadcasts::<super::DkgRound1Message>(&config.session_id, 1, config.n_parties)
        .await?;

    // Round 2: Send secret shares to each party
    debug!("DKG Round 2: Secret sharing");
    for party_id in &config.parties {
        if *party_id == config.party_id {
            continue;
        }
        let share = evaluate_polynomial(&secret_poly, *party_id as u64 + 1);
        let share_msg = super::DkgRound2Message {
            from: config.party_id,
            to: *party_id,
            share: share.to_bytes().to_vec(),
        };
        relay
            .send_direct(&config.session_id, 2, *party_id, &share_msg)
            .await?;
    }

    // Collect shares from all parties
    let received_shares = relay
        .collect_direct::<super::DkgRound2Message>(
            &config.session_id,
            2,
            config.party_id,
            config.n_parties - 1,
        )
        .await?;

    // Round 3: Verify shares and compute final key share
    debug!("DKG Round 3: Verification");

    // Verify received shares against commitments
    for share_msg in &received_shares {
        verify_share(
            share_msg,
            &all_commitments[share_msg.from].commitments,
            config.party_id,
        )?;
    }

    // Compute final secret share
    let mut final_secret = evaluate_polynomial(&secret_poly, config.party_id as u64 + 1);
    for share_msg in &received_shares {
        let share_bytes: [u8; 32] = share_msg
            .share
            .clone()
            .try_into()
            .map_err(|_| Error::Deserialization("Invalid share length".into()))?;
        let share = <Scalar as Reduce<U256>>::reduce_bytes(&share_bytes.into());
        final_secret = final_secret + share;
    }

    // Compute public key
    let public_key = compute_public_key(&all_commitments)?;

    // Compute public shares
    let public_shares = compute_public_shares(&all_commitments, config.n_parties)?;

    // Generate chain code for BIP32
    let chain_code: [u8; 32] = rand::random();

    let key_share = KeyShare {
        party_id: config.party_id,
        n_parties: config.n_parties,
        threshold: config.threshold,
        secret_share: final_secret,
        public_key,
        public_shares,
        chain_code,
    };

    info!(
        party_id = config.party_id,
        public_key = hex::encode(&key_share.public_key),
        "DKG completed successfully"
    );

    Ok(key_share)
}

/// Generate a random secret polynomial of degree t-1
fn generate_secret_polynomial(config: &SessionConfig) -> Result<(Vec<Scalar>, Vec<Vec<u8>>)> {
    let mut rng = OsRng;
    let mut coefficients = Vec::with_capacity(config.threshold);
    let mut commitments = Vec::with_capacity(config.threshold);

    for _ in 0..config.threshold {
        let coef = Scalar::random(&mut rng);
        let commitment = (ProjectivePoint::GENERATOR * coef).to_affine();

        coefficients.push(coef);
        commitments.push(commitment.to_encoded_point(true).as_bytes().to_vec());
    }

    Ok((coefficients, commitments))
}

/// Evaluate polynomial at a point
fn evaluate_polynomial(coefficients: &[Scalar], x: u64) -> Scalar {
    let x_scalar = Scalar::from(x);
    let mut result = Scalar::ZERO;
    let mut x_power = Scalar::ONE;

    for coef in coefficients {
        result = result + (*coef * x_power);
        x_power = x_power * x_scalar;
    }

    result
}

/// Verify a share against commitments
fn verify_share(
    share_msg: &super::DkgRound2Message,
    commitments: &[Vec<u8>],
    my_id: usize,
) -> Result<()> {
    let share_bytes: [u8; 32] = share_msg
        .share
        .clone()
        .try_into()
        .map_err(|_| Error::Deserialization("Invalid share length".into()))?;
    let share = <Scalar as Reduce<U256>>::reduce_bytes(&share_bytes.into());

    // Compute expected commitment
    let expected = ProjectivePoint::GENERATOR * share;

    // Compute actual commitment from Lagrange interpolation of commitments
    let x = (my_id + 1) as u64;
    let mut actual = ProjectivePoint::IDENTITY;
    let mut x_power = Scalar::ONE;
    let x_scalar = Scalar::from(x);

    for commitment_bytes in commitments {
        let point = k256::EncodedPoint::from_bytes(commitment_bytes)
            .map_err(|e| Error::VerificationFailed(e.to_string()))?;
        let affine_opt = AffinePoint::from_encoded_point(&point);
        let affine: AffinePoint = Option::<AffinePoint>::from(affine_opt)
            .ok_or_else(|| Error::VerificationFailed("Invalid commitment point".into()))?;
        let commitment = ProjectivePoint::from(affine);

        actual = actual + (commitment * x_power);
        x_power = x_power * x_scalar;
    }

    if expected != actual {
        return Err(Error::VerificationFailed(format!(
            "Share from party {} does not match commitment",
            share_msg.from
        )));
    }

    Ok(())
}

/// Compute the public key from commitments
fn compute_public_key(all_commitments: &[super::DkgRound1Message]) -> Result<Vec<u8>> {
    let mut public_key = ProjectivePoint::IDENTITY;

    for commitment_msg in all_commitments {
        if commitment_msg.commitments.is_empty() {
            return Err(Error::VerificationFailed("Empty commitments".into()));
        }

        let point = k256::EncodedPoint::from_bytes(&commitment_msg.commitments[0])
            .map_err(|e| Error::VerificationFailed(e.to_string()))?;
        let affine_opt = AffinePoint::from_encoded_point(&point);
        let affine: AffinePoint = Option::<AffinePoint>::from(affine_opt)
            .ok_or_else(|| Error::VerificationFailed("Invalid commitment point".into()))?;
        let commitment = ProjectivePoint::from(affine);

        public_key = public_key + commitment;
    }

    let encoded = public_key.to_affine().to_encoded_point(true);
    Ok(encoded.as_bytes().to_vec())
}

/// Compute public shares for all parties
fn compute_public_shares(
    all_commitments: &[super::DkgRound1Message],
    n_parties: usize,
) -> Result<Vec<Vec<u8>>> {
    let mut public_shares = Vec::with_capacity(n_parties);

    for party_id in 0..n_parties {
        let x = (party_id + 1) as u64;
        let mut public_share = ProjectivePoint::IDENTITY;

        for commitment_msg in all_commitments {
            let mut x_power = Scalar::ONE;
            let x_scalar = Scalar::from(x);

            for commitment_bytes in &commitment_msg.commitments {
                let point = k256::EncodedPoint::from_bytes(commitment_bytes)
                    .map_err(|e| Error::VerificationFailed(e.to_string()))?;
                let affine_opt = AffinePoint::from_encoded_point(&point);
                let affine: AffinePoint = Option::<AffinePoint>::from(affine_opt)
                    .ok_or_else(|| Error::VerificationFailed("Invalid commitment point".into()))?;
                let commitment = ProjectivePoint::from(affine);

                public_share = public_share + (commitment * x_power);
                x_power = x_power * x_scalar;
            }
        }

        let encoded = public_share.to_affine().to_encoded_point(true);
        public_shares.push(encoded.as_bytes().to_vec());
    }

    Ok(public_shares)
}
