use std::collections::HashMap;
use std::path::PathBuf;

use sunder_core::{
    error::SunderError,
    types::{KeyInfo, PartialSigResponse},
};

// Thetacrypt imports — these are the exact types confirmed from the codebase audit
use theta_schemes::interface::{
    Serializable, ThresholdSignature, ThresholdSignatureParams,
};
use theta_schemes::keys::{
    key_store::KeyStore,
    keys::{PrivateKeyShare, PublicKey},
};

/// Holds all key shares for this node
/// Loaded once at startup from the .keystore file
pub struct NodeSigner {
    pub node_index: usize,
    /// key_id (from thetacrypt keygen) → PrivateKeyShare
    keys: HashMap<String, PrivateKeyShare>,
}

impl NodeSigner {
    /// Load all signing key shares from a thetacrypt .keystore file
    pub fn load(keystore_path: &str, node_index: usize) -> Result<Self, SunderError> {
        let path = PathBuf::from(keystore_path);

        if !path.exists() {
            return Err(SunderError::KeystoreLoad(format!(
                "keystore file not found: {}",
                keystore_path
            )));
        }

        let keystore = KeyStore::from_file(&path)
            .map_err(|e| SunderError::KeystoreLoad(e.to_string()))?;

        let mut keys: HashMap<String, PrivateKeyShare> = HashMap::new();

        // get_signing_keys() returns Vec<&KeyEntry>
        // KeyEntry.sk is Option<PrivateKeyShare> — confirmed pub
        for entry in keystore.get_signing_keys() {
            if let Some(sk) = &entry.sk {
                let key_id = sk.get_key_id().to_string();
                tracing::info!(
                    "node {} | loaded key '{}' | scheme={:?} | share_id={} | threshold={}",
                    node_index,
                    key_id,
                    sk.get_scheme(),
                    sk.get_share_id(),
                    sk.get_threshold(),
                );
                keys.insert(key_id, sk.clone());
            }
        }

        if keys.is_empty() {
            tracing::warn!(
                "node {} | no signing keys found in keystore '{}'",
                node_index, keystore_path
            );
        } else {
            tracing::info!(
                "node {} | ready | {} key shares loaded",
                node_index,
                keys.len()
            );
        }

        Ok(Self { node_index, keys })
    }

    /// Produce a partial signature for the given message using the named key
    pub fn partial_sign(
        &self,
        key_name: &str,
        message: &[u8],
        label: &[u8],
    ) -> Result<PartialSigResponse, SunderError> {
        let sk = self.keys.get(key_name).ok_or_else(|| {
            SunderError::KeyNotFound(format!(
                "key '{}' not in keystore — available: [{}]",
                key_name,
                self.keys.keys().cloned().collect::<Vec<_>>().join(", ")
            ))
        })?;

        let mut params = ThresholdSignatureParams::new();

        // This is the exact thetacrypt call:
        // partial_sign(msg: &[u8], label: &[u8], secret: &PrivateKeyShare, params: &mut ThresholdSignatureParams)
        let signature_share =
            ThresholdSignature::partial_sign(message, label, sk, &mut params)
                .map_err(|e| SunderError::PartialSignFailed(format!("{:?}", e)))?;

        // SignatureShare implements Serializable — ASN.1 encoding
        let bytes = signature_share
            .to_bytes()
            .map_err(|e| SunderError::Serialization(format!("{:?}", e)))?;

        tracing::info!(
            "node {} | ✓ partial sig | key='{}' | {} bytes",
            self.node_index,
            key_name,
            bytes.len()
        );

        Ok(PartialSigResponse {
            node_index: self.node_index,
            key_name: key_name.to_string(),
            data: hex::encode(bytes),
        })
    }

    /// Returns the public key for a named key — used for verification
    pub fn get_public_key(&self, key_name: &str) -> Result<PublicKey, SunderError> {
        self.keys
            .get(key_name)
            .map(|sk| sk.get_public_key())
            .ok_or_else(|| SunderError::KeyNotFound(key_name.to_string()))
    }

    /// Returns metadata about all loaded keys
    pub fn list_keys(&self) -> Vec<KeyInfo> {
        self.keys
            .iter()
            .map(|(id, sk)| KeyInfo {
                name: id.clone(),
                scheme: format!("{:?}", sk.get_scheme()),
                threshold: sk.get_threshold(),
                share_id: sk.get_share_id(),
            })
            .collect()
    }

    pub fn key_count(&self) -> usize {
        self.keys.len()
    }
}
