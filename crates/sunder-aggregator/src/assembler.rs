use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use sunder_core::{
    error::SunderError,
    types::{CombinedSigResponse, KeyInfo, PartialSigResponse, PartialSignRequest},
};

// Thetacrypt types — confirmed from codebase audit
use theta_schemes::interface::{
    Serializable, Signature, SignatureShare, ThresholdSignature,
};
use theta_schemes::keys::{key_store::KeyStore, keys::PublicKey};

pub struct SunderAssembler {
    pub nodes: Vec<String>,
    pub threshold: usize,
    client: reqwest::Client,
    /// key_id → PublicKey (loaded from keystore at startup)
    public_keys: HashMap<String, PublicKey>,
}

impl SunderAssembler {
    /// Load public keys from keystore and prepare the HTTP client
    pub fn load(
        nodes: Vec<String>,
        threshold: usize,
        keystore_path: &str,
    ) -> Result<Self, SunderError> {
        let path = PathBuf::from(keystore_path);

        if !path.exists() {
            return Err(SunderError::KeystoreLoad(format!(
                "keystore not found: {}",
                keystore_path
            )));
        }

        let keystore = KeyStore::from_file(&path)
            .map_err(|e| SunderError::KeystoreLoad(e.to_string()))?;

        let mut public_keys: HashMap<String, PublicKey> = HashMap::new();

        // get_signing_keys() → Vec<&KeyEntry>
        // KeyEntry.pk is PublicKey (always present, confirmed pub)
        for entry in keystore.get_signing_keys() {
            let key_id = entry.pk.get_key_id().to_string();
            tracing::info!("aggregator | loaded pubkey '{}'", key_id);
            public_keys.insert(key_id, entry.pk.clone());
        }

        if public_keys.is_empty() {
            tracing::warn!(
                "aggregator | no signing keys found in keystore '{}'",
                keystore_path
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| SunderError::Config(e.to_string()))?;

        Ok(Self {
            nodes,
            threshold,
            client,
            public_keys,
        })
    }

    /// Fan out to all nodes, collect threshold partial sigs, assemble final signature
    pub async fn sign(
        &self,
        key_name: &str,
        message: &[u8],
    ) -> Result<CombinedSigResponse, SunderError> {
        let pubkey = self.public_keys.get(key_name).ok_or_else(|| {
            SunderError::KeyNotFound(format!(
                "'{}' — available: [{}]",
                key_name,
                self.public_keys.keys().cloned().collect::<Vec<_>>().join(", ")
            ))
        })?;

        let req_body = PartialSignRequest {
            key_name: key_name.to_string(),
            message: hex::encode(message),
            label: key_name.to_string(),
        };

        // ── Fan out to all nodes in parallel ──────────────────────────────
        let mut handles = vec![];
        for (i, node_url) in self.nodes.iter().enumerate() {
            let client = self.client.clone();
            let url = format!("{}/partial-sign", node_url);
            let body = req_body.clone();
            let node_idx = i + 1;

            handles.push(tokio::spawn(async move {
                let result = client.post(&url).json(&body).send().await;
                (node_idx, result)
            }));
        }

        // ── Collect first `threshold` successful responses ─────────────────
        let mut partial_sigs: Vec<PartialSigResponse> = vec![];
        let mut nodes_participated: Vec<usize> = vec![];

        for handle in handles {
            if partial_sigs.len() >= self.threshold {
                break;
            }

            match handle.await {
                Ok((idx, Ok(resp))) => match resp.json::<PartialSigResponse>().await {
                    Ok(sig) => {
                        tracing::info!("✓ node {} | partial sig received", idx);
                        nodes_participated.push(idx);
                        partial_sigs.push(sig);
                    }
                    Err(e) => tracing::warn!("✗ node {} | bad response body: {}", idx, e),
                },
                Ok((idx, Err(e))) => {
                    tracing::warn!("✗ node {} | unreachable: {}", idx, e);
                }
                Err(e) => {
                    tracing::warn!("task join error: {}", e);
                }
            }
        }

        if partial_sigs.len() < self.threshold {
            return Err(SunderError::InsufficientShares {
                need: self.threshold,
                got: partial_sigs.len(),
            });
        }

        // ── Deserialize SignatureShares: hex → ASN.1 → thetacrypt type ─────
        let mut shares: Vec<SignatureShare> = vec![];
        for p in &partial_sigs {
            let bytes = hex::decode(&p.data)
                .map_err(|e| SunderError::Serialization(e.to_string()))?;

            let share = SignatureShare::from_bytes(&bytes)
                .map_err(|e| SunderError::Serialization(format!("{:?}", e)))?;

            shares.push(share);
        }

        // ── Assemble using thetacrypt ──────────────────────────────────────
        // assemble(shares: &Vec<SignatureShare>, msg: &[u8], pubkey: &PublicKey)
        let signature = ThresholdSignature::assemble(&shares, message, pubkey)
            .map_err(|e| SunderError::AssemblyFailed(format!("{:?}", e)))?;

        let sig_bytes = signature
            .to_bytes()
            .map_err(|e| SunderError::Serialization(format!("{:?}", e)))?;

        tracing::info!(
            "✅ threshold signature assembled | key='{}' | nodes={:?} | {} bytes",
            key_name,
            nodes_participated,
            sig_bytes.len()
        );

        Ok(CombinedSigResponse {
            key_name: key_name.to_string(),
            signature: hex::encode(sig_bytes),
            nodes_participated,
        })
    }

    /// Verify a previously produced signature
    pub fn verify(
        &self,
        key_name: &str,
        signature_hex: &str,
        message: &[u8],
    ) -> Result<bool, SunderError> {
        let pubkey = self.public_keys.get(key_name)
            .ok_or_else(|| SunderError::KeyNotFound(key_name.to_string()))?;

        let sig_bytes = hex::decode(signature_hex)
            .map_err(|e| SunderError::InvalidHex(e.to_string()))?;

        let signature = Signature::from_bytes(&sig_bytes)
            .map_err(|e| SunderError::Serialization(format!("{:?}", e)))?;

        ThresholdSignature::verify(&signature, pubkey, message)
            .map_err(|e| SunderError::AssemblyFailed(format!("{:?}", e)))
    }

    pub fn list_keys(&self) -> Vec<KeyInfo> {
        self.public_keys
            .keys()
            .map(|id| KeyInfo {
                name: id.clone(),
                scheme: "bls04".to_string(),
                threshold: self.threshold as u16,
                share_id: 0, // aggregator doesn't hold shares
            })
            .collect()
    }
}
