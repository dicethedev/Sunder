use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::error::SunderError;

/// Metadata about a key — stored separately from the cryptographic material
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMeta {
    pub name: String,
    pub scheme: String,   // "bls04"
    pub group: String,    // "bls12381"
    pub threshold: usize,
    pub total_nodes: usize,
    pub node_index: usize,
    /// Path to the .keystore file for this node
    pub keystore_path: String,
}

/// Registry of all keys this node knows about
/// Stored at: /config/keys.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyRegistry {
    pub keys: HashMap<String, KeyMeta>,
}

impl KeyRegistry {
    pub fn load(path: &str) -> Result<Self, SunderError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SunderError::KeystoreLoad(e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| SunderError::KeystoreLoad(e.to_string()))
    }

    pub fn get(&self, key_name: &str) -> Result<&KeyMeta, SunderError> {
        self.keys.get(key_name)
            .ok_or_else(|| SunderError::KeyNotFound(key_name.to_string()))
    }
}