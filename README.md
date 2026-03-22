# Sunder

**Self-hosted threshold signing infrastructure.**  
The key is split. It never comes back together.

---

## What is Sunder?

Sunder is a production-ready service layer on top of [Thetacrypt](https://github.com/cryptobern/thetacrypt) — an IC3 research library implementing BLS04, FROST, and other threshold cryptographic schemes in Rust.

When a message is signed through Sunder:

- No single node ever holds the complete private key
- T nodes each contribute a **partial signature** from their key share
- The aggregator combines them into one valid threshold signature
- Compromise one node → you learn nothing about the key
- Take down `N - T` nodes → signing still works

## Architecture
This visual diagram depicting Sunder's layered system components work (showing how Sunder signing system works)

```mermaid
flowchart LR

    %% ===== CLIENT =====
    subgraph CLIENT["Client Layer"]
        APP["Your Application<br/><small>Bridge · Oracle · DAO · Validator</small>"]
    end

    %% ===== AGGREGATOR =====
    subgraph AGG_LAYER["Coordinator Layer"]
        AGG["sunder-aggregator<br/><small>Auth · Key Registry · Fan-out · Threshold Logic · Audit Log</small>"]
    end

    %% ===== NODES =====
    subgraph NODES["Signing Nodes (t-of-n Threshold)"]
        N1["Node 1<br/><small>share_1</small>"]
        N2["Node 2<br/><small>share_2</small>"]
        N3["Node 3<br/><small>share_3</small>"]
        N4["Node 4<br/><small>share_4</small>"]
        N5["Node 5 ❌<br/><small>offline</small>"]
    end

    %% ===== AGGREGATION =====
    subgraph COMPUTE["Cryptographic Execution"]
        AGGREGATE["Signature Aggregation<br/><small>ThresholdSignature::assemble()</small>"]
    end

    %% ===== RESPONSE =====
    subgraph OUTPUT["Result"]
        RESP["Response<br/><small>{ signature, nodes_participated }</small><br/>🔐 Full key never existed"]
    end

    %% ===== FLOW =====
    APP -->|"POST /v1/sign/{key}"| AGG

    AGG -->|fan-out request| N1
    AGG -->|fan-out request| N2
    AGG -->|fan-out request| N3
    AGG -->|fan-out request| N4
    AGG -.->|unreachable| N5

    N1 -->|partial signature| AGGREGATE
    N2 -->|partial signature| AGGREGATE
    N3 -->|partial signature| AGGREGATE
    N4 -->|partial signature| AGGREGATE

    AGGREGATE -->|final signature| RESP
    RESP --> AGG

    %% ===== STYLING =====
    classDef client fill:#E3F2FD,stroke:#1E88E5,stroke-width:2px,color:#0D47A1;
    classDef aggregator fill:#FFF3E0,stroke:#FB8C00,stroke-width:2px,color:#E65100;
    classDef nodes fill:#E8F5E9,stroke:#43A047,stroke-width:2px,color:#1B5E20;
    classDef offline fill:#FFEBEE,stroke:#E53935,stroke-width:2px,stroke-dasharray: 6 4,color:#B71C1C;
    classDef compute fill:#FFFDE7,stroke:#FDD835,stroke-width:2px,color:#F57F17;
    classDef output fill:#ECEFF1,stroke:#546E7A,stroke-width:2px,color:#263238;

    class APP client;
    class AGG aggregator;
    class N1,N2,N3,N4 nodes;
    class N5 offline;
    class AGGREGATE compute;
    class RESP output;
```

<!-- <p align="center">
  <img src="https://github.com/user-attachments/assets/f495cb7d-d8c5-4941-9e08-a0ca51ce5510" />
</p> -->

## Project Structure

Sunder is organized as a modular, multi-crate Rust workspace designed for distributed threshold signing.

```
sunder/
├── crates/
│   ├── sunder-core/        # Shared types, errors, audit log
│   ├── sunder-node/        # Signing node — holds one key share
│   ├── sunder-aggregator/  # Fan-out, collect, assemble
│   └── sunder-cli/         # Operator CLI
├── sdk/
│   └── sunder-client/      # Rust SDK for application integration
├── docker/
│   ├── Dockerfile.node
│   ├── Dockerfile.aggregator
│   └── docker-compose.yml
└── scripts/
    ├── setup.sh            # One-time key generation
    └── demo.sh             # Fault tolerance demo
```

---

## Quickstart

### 1. Prerequisites

- Docker and Docker Compose installed and running
- Rust toolchain (`curl https://sh.rustup.rs -sSf | sh`)
- Git

### 2. Clone Sunder
```bash
git clone https://github.com/dicethedev/sunder
cd sunder
```
Sunder fetches Thetacrypt automatically via Cargo — no manual cloning needed.

### 3. Build the Thetacrypt Docker image

Sunder uses Thetacrypt's tooling to generate key shares. Build the image once:
```bash
# Clone thetacrypt alongside Sunder
cd ..
git clone https://github.com/dicethedev/thetacrypt
cd thetacrypt/demo

# Fix known compatibility issues with modern Rust/Docker
sed -i 's/FROM rust:.*/FROM rust:latest as builder/' Dockerfile
sed -i 's/FROM debian:12.*/FROM debian:trixie-slim/' Dockerfile
sed -i 's/RUN cargo build --release/RUN RUSTFLAGS="--allow dangerous_implicit_autorefs --allow legacy_derive_helpers" cargo build --release/' Dockerfile
sed -i 's/docker-compose/docker compose/g' Makefile

make set-up
make build-docker
```

> **Why these fixes?** Thetacrypt was written against Rust 1.74. Running it in 2026
> requires bumping the base image and suppressing two lint errors that became
> hard errors in newer Rust. These are one-time setup steps.

### 4. Generate key shares

Back in the Sunder directory:
```bash
cd ../../sunder
chmod +x scripts/setup.sh
./scripts/setup.sh
```

This runs Thetacrypt's `thetacli keygen` inside Docker and generates a
**3-of-5 BLS04 threshold key** — 5 key shares distributed across `config/`,
one per node. The complete private key is never assembled.

Expected output:
```
✅ Key shares generated
✅ Server configs generated
✅ Setup complete!
```

### 5. Build Sunder
```bash
RUSTFLAGS="--allow dangerous_implicit_autorefs --allow legacy_derive_helpers" \
  cargo build --release
```

### 6. Start the cluster
```bash
cd docker
docker compose up
```

This starts:
- 5 signing nodes (each holds one key share)
- 1 aggregator (public-facing API, holds only the public key)

All 6 services are healthy when you see:
```
sunder-aggregator  | 🟢 sunder-aggregator ready on 0.0.0.0:8080
sunder-node1       | 🟢 sunder-node 1 ready on 0.0.0.0:9000
...
```

### 7. Sign something
```bash
# Get the key ID generated during setup
curl http://localhost:8080/v1/keys

# Sign a message (message must be hex-encoded)
# "hello" in hex is 68656c6c6f
curl -X POST http://localhost:8080/v1/sign/<key-id> \
  -H "Content-Type: application/json" \
  -d '{"message": "68656c6c6f"}'
```

Response:
```json
{
  "key_name": "abc123...",
  "signature": "9f3a2c...",
  "nodes_participated": [1, 2, 3]
}
```

The full private key was never held by any single process.

### 8. Verify
```bash
curl -X POST http://localhost:8080/v1/verify \
  -H "Content-Type: application/json" \
  -d '{
    "key_name": "<key-id>",
    "signature": "<sig-hex>",
    "message": "68656c6c6f"
  }'
```

Response:
```json
{ "valid": true }
```

---

## Run the demo

The demo script shows the full signing flow including fault tolerance:
```bash
chmod +x scripts/demo.sh
./scripts/demo.sh
```

What it demonstrates:
1. Health check across all 5 nodes
2. Signs a message — all 5 nodes participate
3. Verifies the signature
4. **Kills 2 nodes** — signing still succeeds with the remaining 3
5. Verifies the new signature is also valid
6. Brings the killed nodes back online

The key insight: the same public key verifies both signatures.
The complete private key was never assembled either time.

---

## CLI

```bash
# Build
cargo build --release -p sunder-cli

# Sign
./target/release/sunder sign --key <key-id> --message 68656c6c6f

# Verify  
./target/release/sunder verify \
  --key <key-id> \
  --sig <hex> \
  --message 68656c6c6f

# List keys
./target/release/sunder keys

# Health check
./target/release/sunder health
```

---

## SDK

```rust
use sunder_client::SunderClient;

#[tokio::main]
async fn main() {
    let client = SunderClient::new("http://localhost:8080");

    // Two lines to sign
    let result = client.sign("bridge-signer", b"approve_withdrawal_4821").await.unwrap();

    println!("signature: {}", result.signature);
    println!("nodes:     {:?}", result.nodes_participated);
}
```

---

## API Reference

### `GET /health`
Returns aggregator health.

### `GET /v1/keys`
Lists all threshold keys available for signing.

### `POST /v1/sign/:key_name`
```json
{ "message": "<hex-encoded bytes>" }
```
Returns:
```json
{
  "key_name": "string",
  "signature": "<hex>",
  "nodes_participated": [1, 2, 3]
}
```

### `POST /v1/verify`
```json
{
  "key_name": "string",
  "signature": "<hex>",
  "message": "<hex>"
}
```
Returns:
```json
{ "valid": true }
```

---

### The cryptographic path

```
POST /v1/sign/my-key
  → aggregator fans out to N nodes
    → each node: ThresholdSignature::partial_sign(msg, label, &key_share, &mut params)
    → returns SignatureShare (ASN.1 serialized, hex over HTTP)
  → aggregator collects T shares
  → ThresholdSignature::assemble(&shares, msg, &pubkey) → Signature
  → returns hex-encoded Signature
```

All cryptographic operations are provided by **Thetacrypt** (IC3 research).  
Sunder provides the service layer: HTTP API, deployment, auth, audit logging.

---

## Trust Model

**What Sunder guarantees:**
- The complete signing key never exists in full — not at setup, not during signing
- Compromising `T - 1` nodes reveals no information about the key
- The cluster continues signing if up to `N - T` nodes are offline or compromised

**What Sunder does NOT guarantee (v0.1):**
- Byzantine-fault-tolerant aggregation — the aggregator is trusted
- Distributed Key Generation — keys are generated by a trusted dealer (thetacrypt's `thetacli keygen`)
- Encrypted channels between nodes — signing messages are not sensitive; key shares are distributed offline at setup

These are documented limitations, not bugs. DKG and proactive share refresh are on the roadmap.

---

## Built at

Shape Rotator Virtual Hackathon 2026  
Track: Cryptographic Primitives  
Built on: [Thetacrypt](https://github.com/cryptobern/thetacrypt) by IC3
