//! SoftSpokenOT implementation
//!
//! OT extension protocol from https://eprint.iacr.org/2022/192.pdf

use crate::Result;

/// SoftSpokenOT protocol
pub struct SoftSpokenOT {
    /// Security parameter
    security_param: usize,
    /// Number of OTs to extend
    count: usize,
}

impl SoftSpokenOT {
    /// Create a new SoftSpokenOT instance
    pub fn new(security_param: usize, count: usize) -> Self {
        Self {
            security_param,
            count,
        }
    }

    /// Run the OT extension protocol as sender
    pub fn extend_sender(&self, base_ots: &[([u8; 32], [u8; 32])]) -> Result<Vec<([u8; 32], [u8; 32])>> {
        // Placeholder implementation
        // Real implementation would use the SoftSpokenOT protocol
        let outputs = vec![([0u8; 32], [0u8; 32]); self.count];
        Ok(outputs)
    }

    /// Run the OT extension protocol as receiver
    pub fn extend_receiver(
        &self,
        base_ots: &[[u8; 32]],
        choices: &[bool],
    ) -> Result<Vec<[u8; 32]>> {
        // Placeholder implementation
        let outputs = vec![[0u8; 32]; self.count];
        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_spoken_setup() {
        let ot = SoftSpokenOT::new(128, 1000);
        assert_eq!(ot.security_param, 128);
        assert_eq!(ot.count, 1000);
    }
}
