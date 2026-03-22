use axum::{Router, routing::{get, post}};
use clap::Parser;
use std::sync::Arc;
use sunder_aggregator::AppState;
use sunder_aggregator::assembler::SunderAssembler;
use sunder_aggregator::handler;
use sunder_core::audit::AuditLog;

#[derive(Parser, Debug)]
#[command(
    name = "sunder-aggregator",
    about = "Sunder aggregator — receives sign requests, fans out to nodes, assembles threshold signature"
)]
pub struct Args {
    /// Keystore file containing public keys (for signature assembly)
    #[arg(long, env = "SUNDER_KEYSTORE")]
    pub keystore: String,

    /// Minimum number of partial signatures required (t-of-n)
    #[arg(long, default_value = "3", env = "SUNDER_THRESHOLD")]
    pub threshold: usize,

    /// Comma-separated list of node URLs
    /// Example: http://node1:9000,http://node2:9000,http://node3:9000
    #[arg(
        long,
        value_delimiter = ',',
        default_values = &[
            "http://node1:9000",
            "http://node2:9000",
            "http://node3:9000",
            "http://node4:9000",
            "http://node5:9000",
        ],
        env = "SUNDER_NODES"
    )]
    pub nodes: Vec<String>,

    /// Address to listen on
    #[arg(long, default_value = "0.0.0.0:8080", env = "SUNDER_BIND")]
    pub bind: String,

    /// Path to audit log file
    #[arg(
        long,
        default_value = "/var/log/sunder/aggregator-audit.jsonl",
        env = "SUNDER_AUDIT_LOG"
    )]
    pub audit_log: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("sunder_aggregator=info".parse().unwrap())
        )
        .init();

    let args = Args::parse();

    if let Some(parent) = std::path::Path::new(&args.audit_log).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let audit = AuditLog::new(&args.audit_log);

    tracing::info!(
        "nodes: {:?} | threshold: {}-of-{}",
        args.nodes,
        args.threshold,
        args.nodes.len()
    );

    let assembler = SunderAssembler::load(
        args.nodes.clone(),
        args.threshold,
        &args.keystore,
    ).unwrap_or_else(|e| {
        tracing::error!("failed to load keystore '{}': {}", args.keystore, e);
        std::process::exit(1);
    });

    let state = Arc::new(AppState { assembler, audit });

    let app = Router::new()
        .route("/health",         get(handler::health))
        .route("/v1/sign/:key",   post(handler::sign))
        .route("/v1/verify",      post(handler::verify))
        .route("/v1/keys",        get(handler::list_keys))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&args.bind)
        .await
        .unwrap_or_else(|e| {
            tracing::error!("failed to bind {}: {}", args.bind, e);
            std::process::exit(1);
        });

    tracing::info!("🟢 sunder-aggregator ready on {}", args.bind);
    axum::serve(listener, app).await.unwrap();
}
