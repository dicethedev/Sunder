use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sunder_core::{
    audit::AuditEvent,
    types::{CombinedSigResponse, HealthResponse, KeyInfo, SignRequest},
};
use crate::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        node_index: None,
        keys_loaded: state.assembler.public_keys.len(),
    })
}

pub async fn list_keys(State(state): State<Arc<AppState>>) -> Json<Vec<KeyInfo>> {
    Json(state.assembler.list_keys())
}

pub async fn sign(
    State(state): State<Arc<AppState>>,
    Path(key_name): Path<String>,
    Json(req): Json<SignRequest>,
) -> Result<Json<CombinedSigResponse>, (StatusCode, String)> {
    let message = hex::decode(&req.message).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("invalid message hex: {}", e))
    })?;

    match state.assembler.sign(&key_name, &message).await {
        Ok(result) => {
            state.audit.write(AuditEvent::SignRequest {
                key_name: key_name.clone(),
                message_hex: req.message.clone(),
                nodes_participated: result.nodes_participated.clone(),
                success: true,
            });
            Ok(Json(result))
        }
        Err(e) => {
            tracing::error!("sign failed for key '{}': {}", key_name, e);
            state.audit.write(AuditEvent::SignRequest {
                key_name: key_name.clone(),
                message_hex: req.message.clone(),
                nodes_participated: vec![],
                success: false,
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub key_name: String,
    pub signature: String,
    pub message: String,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
}

pub async fn verify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    let message = hex::decode(&req.message).map_err(|e| {
        (StatusCode::BAD_REQUEST, format!("invalid message hex: {}", e))
    })?;

    match state.assembler.verify(&req.key_name, &req.signature, &message) {
        Ok(valid) => Ok(Json(VerifyResponse { valid })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
