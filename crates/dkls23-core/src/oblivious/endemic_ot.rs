//! Endemic OT implementation
//!
//! Base oblivious transfer protocol from https://eprint.iacr.org/2019/706.pdf

use crate::{Error, Result};
use rand::rngs::OsRng;
use x25519_dalek::{EphemeralSecret, PublicKey};

/// Endemic OT protocol state
pub struct EndemicOT {
    /// Number of OTs to perform
    count: usize,
}

impl EndemicOT {
    /// Create a new Endemic OT instance
    pub fn new(count: usize) -> Self {
        Self { count }
    }

    /// Sender's first message
    pub fn sender_round1(&self) -> Result<(Vec<EphemeralSecret>, Vec<PublicKey>)> {
        let mut secrets = Vec::with_capacity(self.count);
        let mut public_keys = Vec::with_capacity(self.count);

        for _ in 0..self.count {
            let secret = EphemeralSecret::random_from_rng(OsRng);
            let public = PublicKey::from(&secret);
            secrets.push(secret);
            public_keys.push(public);
        }

        Ok((secrets, public_keys))
    }

    /// Receiver's response given choice bits
    pub fn receiver_round1(
        &self,
        sender_keys: &[PublicKey],
        choices: &[bool],
    ) -> Result<(Vec<[u8; 32]>, Vec<PublicKey>)> {
        if sender_keys.len() != self.count || choices.len() != self.count {
            return Err(Error::InvalidConfig("Mismatched OT parameters".into()));
        }

        let mut outputs = Vec::with_capacity(self.count);
        let mut receiver_keys = Vec::with_capacity(self.count);

        for i in 0..self.count {
            let secret = EphemeralSecret::random_from_rng(OsRng);
            let public = PublicKey::from(&secret);

            // Compute shared secret
            let shared = secret.diffie_hellman(&sender_keys[i]);

            // Output depends on choice
            let output = if choices[i] {
                // XOR with sender's key
                let mut out = *shared.as_bytes();
                for (j, byte) in sender_keys[i].as_bytes().iter().enumerate() {
                    out[j] ^= byte;
                }
                out
            } else {
                *shared.as_bytes()
            };

            outputs.push(output);
            receiver_keys.push(public);
        }

        Ok((outputs, receiver_keys))
    }

    /// Sender derives outputs
    pub fn sender_derive(
        &self,
        secrets: &[EphemeralSecret],
        receiver_keys: &[PublicKey],
    ) -> Result<Vec<([u8; 32], [u8; 32])>> {
        if secrets.len() != self.count || receiver_keys.len() != self.count {
            return Err(Error::InvalidConfig("Mismatched OT parameters".into()));
        }

        let mut outputs = Vec::with_capacity(self.count);

        for i in 0..self.count {
            // This is a simplified version - real implementation would use
            // proper key derivation
            let out0 = [0u8; 32]; // Placeholder
            let out1 = [0u8; 32]; // Placeholder
            outputs.push((out0, out1));
        }

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endemic_ot_setup() {
        let ot = EndemicOT::new(10);
        let (secrets, public_keys) = ot.sender_round1().unwrap();

        assert_eq!(secrets.len(), 10);
        assert_eq!(public_keys.len(), 10);
    }
}
