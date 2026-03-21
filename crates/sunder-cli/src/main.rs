use clap::{Parser, Subcommand};
use serde_json::json;
use sunder_core::types::{CombinedSigResponse, HealthResponse, KeyInfo};

#[derive(Parser)]
#[command(
    name = "sunder",
    about = "Sunder — self-hosted threshold signing infrastructure\nThe key is split. It never comes back together.",
    version = "0.1.0"
)]
struct Cli {
    /// Aggregator URL
    #[arg(long, default_value = "http://localhost:8080", env = "SUNDER_URL")]
    url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sign a message using a named threshold key
    Sign {
        /// Key name to sign with
        #[arg(long)]
        key: String,
        /// Message to sign (hex-encoded bytes)
        #[arg(long)]
        message: String,
    },

    /// Verify a threshold signature
    Verify {
        /// Key name used to produce the signature
        #[arg(long)]
        key: String,
        /// The signature to verify (hex)
        #[arg(long)]
        sig: String,
        /// Original message (hex)
        #[arg(long)]
        message: String,
    },

    /// List all available keys
    Keys,

    /// Check cluster health
    Health,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = reqwest::Client::new();

    match cli.command {
        Commands::Sign { key, message } => {
            println!("🔑 Signing with key '{}'...", key);

            let resp = client
                .post(format!("{}/v1/sign/{}", cli.url, key))
                .json(&json!({ "message": message }))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let result: CombinedSigResponse = r.json().await.unwrap();
                    println!("✅ Signature produced");
                    println!("   key:               {}", result.key_name);
                    println!("   nodes participated: {:?}", result.nodes_participated);
                    println!("   signature:          {}", result.signature);
                }
                Ok(r) => {
                    let status = r.status();
                    let body = r.text().await.unwrap_or_default();
                    eprintln!("❌ Sign failed ({}): {}", status, body);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("❌ Request failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Verify { key, sig, message } => {
            println!("Verifying signature for key '{}'...", key);

            let resp = client
                .post(format!("{}/v1/verify", cli.url))
                .json(&json!({
                    "key_name": key,
                    "signature": sig,
                    "message": message
                }))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let body: serde_json::Value = r.json().await.unwrap();
                    if body["valid"].as_bool().unwrap_or(false) {
                        println!("✅ Signature is valid");
                    } else {
                        println!("❌ Signature is INVALID");
                        std::process::exit(1);
                    }
                }
                Ok(r) => {
                    eprintln!("❌ Verification failed ({})", r.status());
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("❌ Request failed: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Keys => {
            let resp = client
                .get(format!("{}/v1/keys", cli.url))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let keys: Vec<KeyInfo> = r.json().await.unwrap();
                    if keys.is_empty() {
                        println!("No keys found.");
                    } else {
                        println!("{:<30} {:<10} {:<10}", "NAME", "SCHEME", "THRESHOLD");
                        println!("{}", "-".repeat(55));
                        for k in keys {
                            println!("{:<30} {:<10} {}", k.name, k.scheme, k.threshold);
                        }
                    }
                }
                Ok(r) => eprintln!("❌ Failed ({})", r.status()),
                Err(e) => eprintln!("❌ Request failed: {}", e),
            }
        }

        Commands::Health => {
            let resp = client
                .get(format!("{}/health", cli.url))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let health: HealthResponse = r.json().await.unwrap();
                    println!("✅ Aggregator is healthy");
                    println!("   status:      {}", health.status);
                    println!("   keys loaded: {}", health.keys_loaded);
                }
                Ok(r) => eprintln!("❌ Unhealthy ({})", r.status()),
                Err(e) => eprintln!("❌ Cannot reach aggregator: {}", e),
            }
        }
    }
}
