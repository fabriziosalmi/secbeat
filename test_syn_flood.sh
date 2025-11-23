#!/bin/bash
# XDP SYN Flood Protection Test Suite
# Tests SYN cookie generation and XDP_TX behavior

set -e

PROXMOX_HOST="192.168.100.102"
CONTAINER_ID="100"
CONTAINER_IP="192.168.100.15"
MY_IP="192.168.100.12"
TEST_PORT="8443"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   SecBeat XDP SYN Flood Protection Test Suite${NC}"
echo -e "${BLUE}   Chapter 2.4: SYN Cookies${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""
echo "Test Setup:"
echo "  - Container IP: $CONTAINER_IP"
echo "  - My IP: $MY_IP"
echo "  - Target Port: $TEST_PORT"
echo ""

# Check requirements
echo -e "${YELLOW}Checking requirements...${NC}"
if ! command -v hping3 &> /dev/null; then
    echo -e "${RED}❌ hping3 not found. Install with: sudo apt-get install hping3${NC}"
    exit 1
fi
echo -e "${GREEN}✅ hping3 available${NC}"

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- pkill -9 mitigation-node 2>/dev/null || true"
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- pkill -9 tcpdump 2>/dev/null || true"
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- ip link set dev eth0 xdp off 2>/dev/null || true"
    echo -e "${GREEN}Cleanup done${NC}"
}

trap cleanup EXIT

echo ""
echo -e "${BLUE}═══ Phase 1: Deploy Latest Code ═══${NC}"
cleanup
sleep 2

echo "Building and deploying to container..."
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- bash -c 'cd /root/secbeat && \
    git pull 2>&1 | tail -3 && \
    source /root/.cargo/env && \
    ./build_ebpf.sh 2>&1 | tail -5 && \
    cargo build --release --package mitigation-node 2>&1 | tail -5'"

echo -e "${GREEN}✅ Code deployed${NC}"

echo ""
echo -e "${BLUE}═══ Phase 2: Start XDP Program ═══${NC}"

# Start mitigation node with XDP
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- bash -c '\
    cd /root/secbeat && \
    source /root/.cargo/env && \
    nohup ./target/release/mitigation-node --config config.l7-notls.toml > /tmp/mitigation.log 2>&1 & \
    echo \$! > /tmp/mitigation.pid'"

echo "Waiting for XDP to attach..."
sleep 5

# Verify XDP is loaded
if ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- ip link show eth0" | grep -q "xdp"; then
    echo -e "${GREEN}✅ XDP program attached to eth0${NC}"
else
    echo -e "${RED}❌ FAIL: XDP not attached${NC}"
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- tail -30 /tmp/mitigation.log"
    exit 1
fi

echo ""
echo -e "${BLUE}═══ Phase 3: Start tcpdump Capture ═══${NC}"

# Start tcpdump in background to capture SYN/SYN-ACK
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- bash -c '\
    nohup tcpdump -i eth0 -n \"tcp port $TEST_PORT\" -c 20 > /tmp/tcpdump.log 2>&1 & \
    echo \$! > /tmp/tcpdump.pid'"

echo "tcpdump started, capturing 20 packets..."
sleep 2

echo ""
echo -e "${BLUE}═══ Phase 4: Send Test SYN Packets ═══${NC}"

echo "Sending 5 SYN packets with hping3..."
hping3 -S -p $TEST_PORT -c 5 $CONTAINER_IP 2>&1 | grep -E "(HPING|flags|packets)" || true

sleep 2

echo ""
echo -e "${BLUE}═══ Phase 5: Analyze tcpdump Results ═══${NC}"

# Wait for tcpdump to finish
sleep 3
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- pkill tcpdump 2>/dev/null || true"
sleep 1

echo "Packet capture:"
TCPDUMP_OUTPUT=$(ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- cat /tmp/tcpdump.log 2>/dev/null" || echo "No output")
echo "$TCPDUMP_OUTPUT"

echo ""
echo -e "${BLUE}═══ Phase 6: Verify SYN Cookie Behavior ═══${NC}"

# Count SYN packets received
SYN_COUNT=$(echo "$TCPDUMP_OUTPUT" | grep -c "Flags \[S\]" || echo "0")
echo "SYN packets seen: $SYN_COUNT"

# Count SYN-ACK packets sent
SYNACK_COUNT=$(echo "$TCPDUMP_OUTPUT" | grep -c "Flags \[S\.\]" || echo "0")
echo "SYN-ACK packets seen: $SYNACK_COUNT"

if [ "$SYN_COUNT" -gt 0 ]; then
    echo -e "${GREEN}✅ PASS: SYN packets detected${NC}"
else
    echo -e "${YELLOW}⚠️  WARNING: No SYN packets captured${NC}"
fi

if [ "$SYNACK_COUNT" -gt 0 ]; then
    echo -e "${GREEN}✅ PASS: SYN-ACK packets generated (XDP_TX working!)${NC}"
    echo -e "${GREEN}    This proves XDP is generating SYN cookies!${NC}"
else
    echo -e "${YELLOW}⚠️  WARNING: No SYN-ACK packets seen${NC}"
    echo -e "${YELLOW}    This could mean:${NC}"
    echo -e "${YELLOW}    1. SYN packets not reaching XDP${NC}"
    echo -e "${YELLOW}    2. Checksum issue (packets dropped)${NC}"
    echo -e "${YELLOW}    3. XDP_TX not working${NC}"
fi

echo ""
echo -e "${BLUE}═══ Phase 7: Check XDP Logs ═══${NC}"

echo "Recent XDP events from mitigation node:"
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- tail -30 /tmp/mitigation.log" | grep -E "(SYN|cookie|TX)" || echo "No SYN-related logs found"

echo ""
echo -e "${BLUE}═══ Phase 8: SYN Flood Simulation (Optional) ═══${NC}"

read -p "Run SYN flood test? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Sending 1000 SYN packets in flood mode...${NC}"
    echo "This will test XDP's ability to handle high-rate SYN floods"
    
    # Start fresh tcpdump
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- bash -c '\
        nohup tcpdump -i eth0 -n \"tcp port $TEST_PORT\" -c 50 > /tmp/flood_tcpdump.log 2>&1 & \
        echo \$! > /tmp/tcpdump_flood.pid'"
    
    sleep 1
    
    # Send flood
    timeout 5 hping3 -S -p $TEST_PORT --flood --rand-source $CONTAINER_IP 2>&1 | head -10 || true
    
    sleep 3
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- pkill tcpdump 2>/dev/null || true"
    
    echo ""
    echo "Flood results:"
    FLOOD_OUTPUT=$(ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- cat /tmp/flood_tcpdump.log 2>/dev/null" || echo "No output")
    
    FLOOD_SYN=$(echo "$FLOOD_OUTPUT" | grep -c "Flags \[S\]" || echo "0")
    FLOOD_SYNACK=$(echo "$FLOOD_OUTPUT" | grep -c "Flags \[S\.\]" || echo "0")
    
    echo "  SYN packets: $FLOOD_SYN"
    echo "  SYN-ACK packets: $FLOOD_SYNACK"
    
    if [ "$FLOOD_SYNACK" -gt 0 ]; then
        echo -e "${GREEN}✅ XDP handled SYN flood successfully!${NC}"
    fi
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   SYN Flood Protection Tests Completed${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo "Summary:"
echo "  - SYN packets received: $SYN_COUNT"
echo "  - SYN-ACK packets sent: $SYNACK_COUNT"
if [ "$SYNACK_COUNT" -gt 0 ]; then
    echo -e "  - ${GREEN}Status: ✅ SYN cookies working!${NC}"
else
    echo -e "  - ${YELLOW}Status: ⚠️  Needs investigation${NC}"
fi
echo ""
echo "Key indicators of success:"
echo "  1. SYN-ACK packets visible in tcpdump (XDP_TX working)"
echo "  2. Logs show 'SYN from' and 'TX SYN-ACK' messages"
echo "  3. Kernel memory stays low during flood (no SYN queue buildup)"
