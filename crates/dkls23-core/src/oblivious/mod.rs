//! Oblivious Transfer (OT) primitives
//!
//! This module provides OT protocols used in the DKLs23 signing protocol:
//! - Endemic OT (base OT)
//! - SoftSpokenOT (OT extension)

pub mod endemic_ot;
pub mod soft_spoken;

pub use endemic_ot::EndemicOT;
pub use soft_spoken::SoftSpokenOT;
