use axum::{
    Router,
    routing::{get, post},
};
use clap::Parser;
use std::sync::Arc;
use sunder_core::audit::{AuditEvent, AuditLog};
use sunder_node::{AppState, handler, node_signer::NodeSigner};

#[derive(Parser, Debug)]
#[command(
    name = "sunder-node",
    about = "Sunder signing node — holds a key share and produces partial signatures"
)]
pub struct Args {
    /// Path to the .keystore file for this node
    #[arg(long, env = "SUNDER_KEYSTORE")]
    pub keystore: String,

    /// This node's index (1-based, must be unique in the cluster)
    #[arg(long, env = "SUNDER_NODE_INDEX")]
    pub node_index: usize,

    /// Address to listen on
    #[arg(long, default_value = "0.0.0.0:9000", env = "SUNDER_BIND")]
    pub bind: String,

    /// Path to audit log file
    #[arg(
        long,
        default_value = "/var/log/sunder/node-audit.jsonl",
        env = "SUNDER_AUDIT_LOG"
    )]
    pub audit_log: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sunder_node=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    // Create audit log directory if needed
    if let Some(parent) = std::path::Path::new(&args.audit_log).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let audit = AuditLog::new(&args.audit_log);

    let signer = NodeSigner::load(&args.keystore, args.node_index).unwrap_or_else(|e| {
        tracing::error!("failed to load keystore '{}': {}", args.keystore, e);
        std::process::exit(1);
    });

    // Log all loaded keys to audit
    for key_info in signer.list_keys() {
        audit.write(AuditEvent::KeyLoaded {
            key_name: key_info.name.clone(),
            scheme: key_info.scheme.clone(),
        });
    }

    audit.write(AuditEvent::NodeStarted {
        node_index: args.node_index,
        bind_addr: args.bind.clone(),
    });

    let state = Arc::new(AppState { signer, audit });

    let app = Router::new()
        .route("/health", get(handler::health))
        .route("/partial-sign", post(handler::partial_sign))
        .route("/keys", get(handler::list_keys))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&args.bind)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("failed to bind to {}: {}", args.bind, e);
            std::process::exit(1);
        });

    tracing::info!("🟢 sunder-node {} ready on {}", args.node_index, args.bind);

    axum::serve(listener, app).await.unwrap();
}
