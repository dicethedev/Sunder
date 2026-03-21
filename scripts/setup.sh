#!/usr/bin/env bash
# setup.sh
# Generates a 3-of-5 threshold key using thetacrypt's keygen tooling
# and places keystores in config/ for the Docker cluster to mount.
#
# Run this ONCE before starting the cluster:
#   chmod +x scripts/setup.sh && ./scripts/setup.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CONFIG_DIR="$ROOT_DIR/config"
THETACRYPT_DIR="$ROOT_DIR/../thetacrypt"
DEMO_DIR="$THETACRYPT_DIR/demo"

echo "═══════════════════════════════════════════════════════"
echo "  Sunder — Key Setup"
echo "  Generating 3-of-5 BLS04 threshold key shares"
echo "═══════════════════════════════════════════════════════"

# ── 1. Verify thetacrypt demo image exists ───────────────────────────────────
if ! docker image inspect rust-threshold-library &>/dev/null; then
    echo ""
    echo "⚠  The thetacrypt Docker image is not built yet."
    echo "   Build it first:"
    echo ""
    echo "     cd $DEMO_DIR"
    echo "     make build-docker"
    echo ""
    exit 1
fi

# ── 2. Create config directory ───────────────────────────────────────────────
mkdir -p "$CONFIG_DIR"
chmod 777 "$CONFIG_DIR"

echo ""
echo "📁 Config directory: $CONFIG_DIR"

# ── 3. Write server IPs for a 5-node setup ──────────────────────────────────
cat > "$CONFIG_DIR/server_ips.txt" << 'EOF'
127.0.0.1
127.0.0.1
127.0.0.1
127.0.0.1
127.0.0.1
EOF

echo "✅ server_ips.txt written"

# ── 4. Generate key shares using thetacrypt ──────────────────────────────────
# This runs inside Docker so it uses the correctly compiled thetacrypt binary.
# -k=3  → threshold (min nodes to sign)
# -n=5  → total nodes
# --subjects Bls04-Bls12381 → BLS04 scheme on BLS12-381 curve

echo ""
echo "🔑 Generating BLS04 key shares (3-of-5)..."

docker run --rm \
    -v "$CONFIG_DIR":/target/release/conf:Z \
    rust-threshold-library \
    ./thetacli keygen \
        -k=3 \
        -n=5 \
        --subjects Bls04-Bls12381 \
        --output ./conf \
        --new

echo "✅ Key shares generated"

# ── 5. Generate server config files ─────────────────────────────────────────
echo ""
echo "⚙  Generating server config files..."

docker run --rm \
    -v "$CONFIG_DIR":/target/release/conf:Z \
    rust-threshold-library \
    ./confgen \
        --ip-file conf/server_ips.txt \
        --port-strategy consecutive \
        --outdir conf

echo "✅ Server configs generated"

# ── 6. Verify output ─────────────────────────────────────────────────────────
echo ""
echo "📦 Files in config/:"
ls -la "$CONFIG_DIR"

# Check we have all expected keystores
MISSING=0
for i in 1 2 3 4 5; do
    if [ ! -f "$CONFIG_DIR/node${i}.keystore" ]; then
        echo "❌ Missing: node${i}.keystore"
        MISSING=1
    fi
done

if [ $MISSING -eq 0 ]; then
    echo ""
    echo "═══════════════════════════════════════════════════════"
    echo "  ✅ Setup complete!"
    echo ""
    echo "  Start the cluster:"
    echo "    cd docker && docker compose up"
    echo ""
    echo "  Sign a message:"
    echo "    sunder sign --key <key-id> --message <hex>"
    echo ""
    echo "  Or via curl:"
    echo "    curl -X POST http://localhost:8080/v1/sign/<key-id> \\"
    echo "      -H 'Content-Type: application/json' \\"
    echo "      -d '{\"message\": \"68656c6c6f\"}'"
    echo "═══════════════════════════════════════════════════════"
else
    echo ""
    echo "❌ Setup incomplete — some keystores are missing."
    echo "   Check the output above for errors."
    exit 1
fi
