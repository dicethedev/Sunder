//! # sunder-client
//!
//! The Sunder SDK — integrate threshold signing in two lines of Rust.
//!
//! ## Example
//!
//! ```rust,no_run
//! use sunder_client::SunderClient;
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = SunderClient::new("http://localhost:8080");
//!     let result = client.sign("bridge-signer", b"approve_withdrawal_4821").await.unwrap();
//!     println!("signature: {}", result.signature);
//! }
//! ```

use sunder_core::{error::SunderError, types::CombinedSigResponse};

pub struct SunderClient {
    base_url: String,
    client: reqwest::Client,
}

impl SunderClient {
    /// Create a new Sunder client pointing at an aggregator
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// Sign a message using a named threshold key.
    ///
    /// The message is automatically hex-encoded for transport.
    /// Returns the combined threshold signature and which nodes participated.
    pub async fn sign(
        &self,
        key_name: &str,
        message: &[u8],
    ) -> Result<CombinedSigResponse, SunderError> {
        let resp = self
            .client
            .post(format!("{}/v1/sign/{}", self.base_url, key_name))
            .json(&serde_json::json!({ "message": hex::encode(message) }))
            .send()
            .await
            .map_err(|e| SunderError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(SunderError::Http(format!("{}: {}", status, body)));
        }

        resp.json::<CombinedSigResponse>()
            .await
            .map_err(|e| SunderError::Http(e.to_string()))
    }

    /// Verify a previously produced threshold signature
    pub async fn verify(
        &self,
        key_name: &str,
        signature_hex: &str,
        message: &[u8],
    ) -> Result<bool, SunderError> {
        let resp = self
            .client
            .post(format!("{}/v1/verify", self.base_url))
            .json(&serde_json::json!({
                "key_name": key_name,
                "signature": signature_hex,
                "message": hex::encode(message)
            }))
            .send()
            .await
            .map_err(|e| SunderError::Http(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SunderError::Http(e.to_string()))?;

        Ok(body["valid"].as_bool().unwrap_or(false))
    }
}
