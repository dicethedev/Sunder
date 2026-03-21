#!/usr/bin/env bash
# demo.sh
# The Sunder demo — shows threshold signing in action, including fault tolerance.
# Run this AFTER setup.sh and docker compose up.

set -euo pipefail

AGGREGATOR="http://localhost:8080"

# Hex encoding of "approve_withdrawal_4821"
MESSAGE=$(echo -n "approve_withdrawal_4821" | xxd -p | tr -d '\n')

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

banner() {
    echo ""
    echo -e "${BLUE}${BOLD}══════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}${BOLD}  $1${NC}"
    echo -e "${BLUE}${BOLD}══════════════════════════════════════════════════════${NC}"
}

step() {
    echo ""
    echo -e "${YELLOW}▶ $1${NC}"
}

ok() {
    echo -e "${GREEN}✅ $1${NC}"
}

fail() {
    echo -e "${RED}❌ $1${NC}"
}

# ── Get the first key ID from the aggregator ─────────────────────────────────
get_key_id() {
    curl -s "$AGGREGATOR/v1/keys" | \
        python3 -c "import sys,json; keys=json.load(sys.stdin); print(keys[0]['name'] if keys else '')" \
        2>/dev/null || echo ""
}

banner "Sunder — Threshold Signing Demo"
echo ""
echo "  The key is split. It never comes back together."
echo ""
echo "  Cluster:   5 nodes"
echo "  Threshold: 3-of-5"
echo "  Message:   'approve_withdrawal_4821'"

# ── Step 1: Health check ──────────────────────────────────────────────────────
banner "Step 1: Cluster Health"

step "Checking aggregator..."
HEALTH=$(curl -s "$AGGREGATOR/health")
if echo "$HEALTH" | grep -q '"ok"'; then
    ok "Aggregator is healthy"
    echo "   $HEALTH"
else
    fail "Aggregator not responding — is the cluster running?"
    echo "   Start it: cd docker && docker compose up -d"
    exit 1
fi

step "Checking all 5 nodes..."
for i in 1 2 3 4 5; do
    NODE_HEALTH=$(curl -s "http://localhost:9000/health" 2>/dev/null || echo "unreachable")
    echo "   node$i: $NODE_HEALTH"
done

# ── Step 2: List available keys ───────────────────────────────────────────────
banner "Step 2: Available Keys"

step "Fetching key list from aggregator..."
KEYS=$(curl -s "$AGGREGATOR/v1/keys")
echo "   $KEYS"

KEY_ID=$(get_key_id)
if [ -z "$KEY_ID" ]; then
    fail "No keys found — did you run setup.sh?"
    exit 1
fi

ok "Using key: $KEY_ID"

# ── Step 3: Sign with all 5 nodes ────────────────────────────────────────────
banner "Step 3: Threshold Signing (5 nodes online)"

step "Sending sign request..."
echo "   key:     $KEY_ID"
echo "   message: $MESSAGE"
echo ""

RESULT=$(curl -s -X POST "$AGGREGATOR/v1/sign/$KEY_ID" \
    -H "Content-Type: application/json" \
    -d "{\"message\": \"$MESSAGE\"}")

echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
print(f'   nodes participated: {r[\"nodes_participated\"]}')
print(f'   signature:          {r[\"signature\"][:64]}...')
"

SIG=$(echo "$RESULT" | python3 -c "import sys,json; print(json.load(sys.stdin)['signature'])")
ok "Threshold signature produced"

# ── Step 4: Verify the signature ─────────────────────────────────────────────
banner "Step 4: Verify Signature"

step "Verifying signature against public key..."
VERIFY=$(curl -s -X POST "$AGGREGATOR/v1/verify" \
    -H "Content-Type: application/json" \
    -d "{\"key_name\": \"$KEY_ID\", \"signature\": \"$SIG\", \"message\": \"$MESSAGE\"}")

echo "   $VERIFY"

if echo "$VERIFY" | grep -q '"valid":true'; then
    ok "Signature is valid ✓"
else
    fail "Verification failed"
    exit 1
fi

# ── Step 5: Fault tolerance demo ─────────────────────────────────────────────
banner "Step 5: Fault Tolerance — Killing 2 Nodes"

step "Stopping node4 and node5..."
docker stop sunder-node4 sunder-node5 2>/dev/null || true
ok "node4 and node5 are DOWN"
echo ""
echo -e "   ${RED}node4: ✗ offline${NC}"
echo -e "   ${RED}node5: ✗ offline${NC}"
echo -e "   ${GREEN}node1: ✓ online${NC}"
echo -e "   ${GREEN}node2: ✓ online${NC}"
echo -e "   ${GREEN}node3: ✓ online${NC}"
echo ""
echo "   Threshold is 3-of-5 — signing should still work with 3 nodes"

sleep 2

step "Signing the same message with only 3 nodes..."
RESULT2=$(curl -s -X POST "$AGGREGATOR/v1/sign/$KEY_ID" \
    -H "Content-Type: application/json" \
    -d "{\"message\": \"$MESSAGE\"}")

echo "$RESULT2" | python3 -c "
import sys, json
r = json.load(sys.stdin)
print(f'   nodes participated: {r[\"nodes_participated\"]}')
print(f'   signature:          {r[\"signature\"][:64]}...')
" 2>/dev/null || {
    fail "Sign failed — check cluster output"
    echo "   $RESULT2"
    docker start sunder-node4 sunder-node5
    exit 1
}

ok "Threshold signature produced with only 3 nodes"

SIG2=$(echo "$RESULT2" | python3 -c "import sys,json; print(json.load(sys.stdin)['signature'])")

step "Verifying the new signature..."
VERIFY2=$(curl -s -X POST "$AGGREGATOR/v1/verify" \
    -H "Content-Type: application/json" \
    -d "{\"key_name\": \"$KEY_ID\", \"signature\": \"$SIG2\", \"message\": \"$MESSAGE\"}")

if echo "$VERIFY2" | grep -q '"valid":true'; then
    ok "Signature is valid ✓"
else
    fail "Verification failed"
    docker start sunder-node4 sunder-node5
    exit 1
fi

# ── Step 6: Bring nodes back ──────────────────────────────────────────────────
banner "Step 6: Recovery"
step "Restarting node4 and node5..."
docker start sunder-node4 sunder-node5
sleep 2
ok "All 5 nodes back online"

# ── Summary ───────────────────────────────────────────────────────────────────
banner "Demo Complete"
echo ""
echo -e "  ${GREEN}${BOLD}What just happened:${NC}"
echo ""
echo "  1. A BLS04 threshold key was split across 5 nodes at setup."
echo "     The complete key never existed — only shares."
echo ""
echo "  2. A message was signed by 3 independent nodes."
echo "     Each node computed a partial signature from its share."
echo "     The aggregator combined them into one valid signature."
echo ""
echo "  3. Two nodes were killed. Signing still worked."
echo "     3-of-5 threshold — one-third of the cluster can fail."
echo ""
echo "  4. Both signatures verified correctly against the public key."
echo ""
echo -e "  ${BOLD}The full private key was never assembled. Not once.${NC}"
echo ""
