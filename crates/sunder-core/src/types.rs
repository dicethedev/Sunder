use serde::{Deserialize, Serialize};

/// Request sent from aggregator → each node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSignRequest {
    /// Named key to sign with (must exist in node's keystore)
    pub key_name: String,
    /// Hex-encoded message bytes
    pub message: String,
    /// Label for the signing operation (used by BLS04 internally)
    pub label: String,
}

/// Response from a node after partial signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSigResponse {
    /// Which node produced this share (1-based index)
    pub node_index: usize,
    /// Key that was used
    pub key_name: String,
    /// Hex-encoded ASN.1 serialized SignatureShare (thetacrypt format)
    pub data: String,
}

/// Final combined signature returned to the caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSigResponse {
    pub key_name: String,
    /// Hex-encoded final threshold signature
    pub signature: String,
    /// Which nodes participated in this signing round
    pub nodes_participated: Vec<usize>,
}

/// Request body for POST /v1/sign/:key_name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRequest {
    /// Hex-encoded message bytes to sign
    pub message: String,
}

/// Node health response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_index: Option<usize>,
    pub keys_loaded: usize,
}

/// Key info returned by GET /v1/keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub name: String,
    pub scheme: String,
    pub threshold: u16,
    pub share_id: u16,
}
