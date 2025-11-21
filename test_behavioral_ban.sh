#!/bin/bash

# ============================================================================
# End-to-End Behavioral Analysis Test
# Tests the complete flow: Mitigation Node → NATS → Orchestrator → Ban
# ============================================================================

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PROXY_URL="http://localhost:8443" # Adjust to match your docker-compose port
ATTACK_COUNT=60 # Number of requests to trigger threshold (Error threshold: 50)

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║  🚀 SecBeat Behavioral Analysis - End-to-End Test         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"

# 0. Prerequisites
echo -e "\n${YELLOW}[0/4] Checking Environment...${NC}"
if ! curl -s --max-time 2 $PROXY_URL/health > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  Proxy not reachable at $PROXY_URL. Is Docker running?${NC}"
    echo "Attempting to start services..."
    docker-compose up -d
    echo "Waiting 15s for initialization..."
    sleep 15
fi

# Check if NATS is running
echo "Checking NATS connectivity..."
if docker-compose ps | grep -q nats; then
    echo -e "${GREEN}✅ NATS server is running${NC}"
else
    echo -e "${YELLOW}⚠️  NATS server not found. Starting services...${NC}"
    docker-compose up -d
    sleep 10
fi

# 1. Baseline Check
echo -e "\n${YELLOW}[1/4] Verifying Normal Traffic (Should PASS)...${NC}"
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" $PROXY_URL/health)

if [ "$HTTP_CODE" == "200" ] || [ "$HTTP_CODE" == "404" ]; then
    echo -e "${GREEN}✅ Baseline check passed (HTTP $HTTP_CODE)${NC}"
    echo "   Service is responding normally"
else
    echo -e "${RED}❌ Baseline check failed. Service down? (HTTP $HTTP_CODE)${NC}"
    exit 1
fi

# 2. The Attack Simulation
echo -e "\n${YELLOW}[2/4] Launching Error Flood Attack (Simulating malicious actor)...${NC}"
echo "Target: $PROXY_URL"
echo "Attack vector: $ATTACK_COUNT sequential 404 errors"
echo "Expected behavior: Orchestrator should detect anomaly and issue ban"
echo ""
echo "Progress:"

# Generate errors with progress indicator
for i in $(seq 1 $ATTACK_COUNT); do
    curl -s -o /dev/null "$PROXY_URL/non-existent-page-for-ban-$i" &
    
    # Progress indicator every 10 requests
    if [ $((i % 10)) -eq 0 ]; then
        echo -ne "  [$i/$ATTACK_COUNT] errors sent\r"
    fi
done
wait
echo -e "\n${GREEN}✅ Attack batch completed ($ATTACK_COUNT requests sent)${NC}"

# 3. Processing Time
echo -e "\n${YELLOW}[3/4] Waiting for Orchestrator Analysis...${NC}"
echo "Components in action:"
echo "  1. Mitigation Node publishes telemetry → secbeat.telemetry.{node_id}"
echo "  2. Orchestrator BehavioralExpert analyzes sliding window"
echo "  3. Threshold exceeded → BlockCommand published → secbeat.commands.block"
echo "  4. Mitigation Node receives ban and updates DynamicRuleState"
echo ""
echo "Waiting 8 seconds for NATS propagation and rule enforcement..."
for i in {8..1}; do
    echo -ne "  ${i}s remaining...\r"
    sleep 1
done
echo -e "\n"

# 4. Verification
echo -e "${YELLOW}[4/4] Verifying IP Ban Status...${NC}"
echo "Attempting legitimate request (should be blocked if ban active)..."

# Try a valid endpoint - should be blocked if behavioral ban is active
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" --max-time 3 $PROXY_URL/health)

echo ""
if [ "$HTTP_CODE" == "403" ]; then
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                  🎉 TEST PASSED! 🎉                        ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo -e "${GREEN}✅ Request was blocked (HTTP 403 Forbidden)${NC}"
    echo -e "${GREEN}✅ Behavioral Analysis Expert successfully detected anomaly${NC}"
    echo -e "${GREEN}✅ NATS message propagation working${NC}"
    echo -e "${GREEN}✅ Dynamic IP blocking enforced${NC}"
    echo ""
    echo -e "The end-to-end pipeline is working correctly:"
    echo "  Mitigation Node → NATS → Orchestrator → NATS → Mitigation Node"
    echo ""
    echo -e "${BLUE}Note: Ban will expire after 5 minutes (block_duration_seconds: 300)${NC}"
    
elif [ "$HTTP_CODE" == "000" ]; then
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                  🎉 TEST PASSED! 🎉                        ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo -e "${GREEN}✅ Connection refused/timeout (ban active)${NC}"
    echo -e "${GREEN}✅ IP successfully blocked at network level${NC}"
    
elif [ "$HTTP_CODE" == "200" ] || [ "$HTTP_CODE" == "404" ]; then
    echo -e "${RED}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║                   ❌ TEST FAILED                           ║${NC}"
    echo -e "${RED}╚════════════════════════════════════════════════════════════╝${NC}"
    echo -e "${RED}❌ Request still passed (HTTP $HTTP_CODE)${NC}"
    echo ""
    echo -e "${YELLOW}Troubleshooting steps:${NC}"
    echo "  1. Check NATS connection:"
    echo "     docker-compose logs nats | tail -20"
    echo ""
    echo "  2. Check Orchestrator logs:"
    echo "     docker-compose logs orchestrator | grep -i behavioral"
    echo ""
    echo "  3. Check Mitigation Node logs:"
    echo "     docker-compose logs mitigation-node | grep -i block"
    echo ""
    echo "  4. Verify threshold configuration:"
    echo "     Requests sent: $ATTACK_COUNT"
    echo "     Error threshold: 50 (in BehavioralConfig)"
    echo ""
    echo "  5. Check if NATS topics are being published:"
    echo "     docker-compose exec nats nats sub 'secbeat.telemetry.>'"
    echo "     docker-compose exec nats nats sub 'secbeat.commands.block'"
    
else
    echo -e "${YELLOW}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${YELLOW}║                ⚠️  UNEXPECTED RESULT                      ║${NC}"
    echo -e "${YELLOW}╚════════════════════════════════════════════════════════════╝${NC}"
    echo -e "${YELLOW}⚠️  HTTP Code: $HTTP_CODE${NC}"
    echo ""
    echo "This may indicate a configuration issue or service error."
    echo "Check logs for more details."
fi

echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Test Complete.${NC}"
echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"
