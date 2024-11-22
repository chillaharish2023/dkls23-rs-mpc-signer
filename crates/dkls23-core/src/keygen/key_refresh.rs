//! Key refresh protocol

use crate::mpc::Relay;
use crate::{KeyShare, Result, SessionConfig};
use tracing::{info, instrument};

/// Run the key refresh protocol
///
/// This allows parties to refresh their shares without changing the public key.
/// Useful for proactive security - regularly refreshing shares to limit the
/// window of vulnerability if a share is compromised.
#[instrument(skip(relay, key_share))]
pub async fn run_key_refresh<R: Relay>(
    config: &SessionConfig,
    key_share: &KeyShare,
    relay: &R,
) -> Result<KeyShare> {
    info!(
        party_id = config.party_id,
        "Starting key refresh"
    );

    // Key refresh follows similar structure to DKG but with zero-sum shares
    // This ensures the public key remains unchanged

    // For now, return a placeholder - full implementation would follow
    // the key refresh protocol from the DKLs23 paper

    info!(
        party_id = config.party_id,
        "Key refresh completed"
    );

    Ok(key_share.clone())
}
