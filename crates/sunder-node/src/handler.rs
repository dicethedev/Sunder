use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use sunder_core::{
    audit::AuditEvent,
    types::{HealthResponse, KeyInfo, PartialSignRequest, PartialSigResponse},
};
use crate::AppState;

pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        node_index: Some(state.signer.node_index),
        keys_loaded: state.signer.key_count(),
    })
}

pub async fn list_keys(State(state): State<Arc<AppState>>) -> Json<Vec<KeyInfo>> {
    Json(state.signer.list_keys())
}

pub async fn partial_sign(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PartialSignRequest>,
) -> Result<Json<PartialSigResponse>, (StatusCode, String)> {
    let message = hex::decode(&req.message).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid message hex: {}", e),
        )
    })?;

    let label = req.label.as_bytes();

    match state.signer.partial_sign(&req.key_name, &message, label) {
        Ok(result) => Ok(Json(result)),
        Err(e) => {
            tracing::error!("partial_sign failed for key '{}': {}", req.key_name, e);
            state.audit.write(AuditEvent::SignFailed {
                key_name: req.key_name.clone(),
                reason: e.to_string(),
            });
            Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        }
    }
}
