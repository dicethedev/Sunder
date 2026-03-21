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

```
Your App
   │
   │  POST /v1/sign/bridge-signer
   ▼
┌──────────────────────────────────────────┐
│  sunder-aggregator                        │
│  Fans out to all nodes in parallel       │
└──────┬──────────┬──────────┬─────────────┘
       ▼          ▼          ▼
  ┌─────────┐ ┌─────────┐ ┌─────────┐  ...
  │ node 1  │ │ node 2  │ │ node 3  │
  │ share_1 │ │ share_2 │ │ share_3 │
  └─────────┘ └─────────┘ └─────────┘
       │          │          │
       └──────────┴──────────┘
                  │
           combine(t partial sigs)
                  │
           ✅ Valid Signature
           (full key never existed)
```

---

## Quickstart

### 1. Prerequisites

- Docker and Docker Compose
- The [thetacrypt](https://github.com/cryptobern/thetacrypt) repository cloned alongside Sunder:

```
Thresh-labs/
├── Sunder/        ← this repo
└── thetacrypt/    ← thetacrypt fork
```

Build the thetacrypt Docker image:

```bash
cd ../thetacrypt/demo
make set-up
make build-docker
```

### 2. Generate key shares

```bash
chmod +x scripts/setup.sh
./scripts/setup.sh
```

This generates a **3-of-5 BLS04 threshold key** using thetacrypt's `thetacli keygen` and places the keystores in `config/`.

### 3. Start the cluster

```bash
cd docker
docker compose up
```

Five signing nodes and one aggregator start up. Each node loads its key share. The aggregator loads the public key.

### 4. Sign something

```bash
# Get the key ID
curl http://localhost:8080/v1/keys

# Sign a message (hex-encoded)
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

### 5. Verify

```bash
curl -X POST http://localhost:8080/v1/verify \
  -H "Content-Type: application/json" \
  -d '{
    "key_name": "<key-id>",
    "signature": "<sig-hex>",
    "message": "68656c6c6f"
  }'
```

---

## Run the demo

```bash
chmod +x scripts/demo.sh
./scripts/demo.sh
```

The demo:
1. Signs a message with all 5 nodes
2. Verifies the signature
3. **Kills 2 nodes** — signing still works with the remaining 3
4. Verifies the new signature
5. Brings the nodes back online

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

## Architecture

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
