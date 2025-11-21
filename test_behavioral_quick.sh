#!/bin/bash
# Quick behavioral ban test - simplified version for rapid development testing

set -e

PROXY_URL="${PROXY_URL:-http://localhost:8443}"
ATTACK_COUNT="${ATTACK_COUNT:-60}"

echo "🧪 Quick Behavioral Ban Test"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Baseline
echo "1. Baseline check..."
curl -s -o /dev/null -w "   Status: %{http_code}\n" $PROXY_URL/health

# Attack
echo "2. Sending $ATTACK_COUNT errors..."
for i in $(seq 1 $ATTACK_COUNT); do
    curl -s -o /dev/null "$PROXY_URL/attack-$i" &
done
wait
echo "   ✓ Done"

# Wait
echo "3. Waiting 5s for analysis..."
sleep 5

# Verify
echo "4. Checking ban status..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time 2 $PROXY_URL/health)

if [ "$HTTP_CODE" == "403" ] || [ "$HTTP_CODE" == "000" ]; then
    echo "   ✅ BANNED (HTTP $HTTP_CODE)"
    exit 0
else
    echo "   ❌ NOT BANNED (HTTP $HTTP_CODE)"
    exit 1
fi
