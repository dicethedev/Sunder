// NOTE: This file uses thetacrypt types directly.
// The actual impl is in sunder-node where thetacrypt is a dependency.
// sunder-core defines the trait contract:

use crate::error::SunderError;
use crate::types::{PartialSigResponse, CombinedSigResponse};

/// Trait that sunder-node implements using thetacrypt
pub trait Signer: Send + Sync {
    fn partial_sign(
        &self,
        key_name: &str,
        message: &[u8],
        label: &[u8],
    ) -> Result<PartialSigResponse, SunderError>;
}

/// Trait that sunder-aggregator implements
pub trait Assembler: Send + Sync {
    fn assemble(
        &self,
        key_name: &str,
        message: &[u8],
        partial_sigs: Vec<PartialSigResponse>,
    ) -> Result<CombinedSigResponse, SunderError>;
}