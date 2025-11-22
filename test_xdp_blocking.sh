#!/bin/bash
# XDP Blocking Test Suite
# Tests kernel-level packet dropping on Proxmox container

set -e

PROXMOX_HOST="192.168.100.102"
CONTAINER_ID="100"
CONTAINER_IP="192.168.100.15"
MY_IP="192.168.100.12"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   SecBeat XDP Blocking Test Suite${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""
echo "Test Setup:"
echo "  - Container IP: $CONTAINER_IP (target)"
echo "  - My IP: $MY_IP (will be blocked)"
echo ""

# Function to test ping
test_ping() {
    local description="$1"
    local should_work="$2"
    
    echo -e "${YELLOW}>>> Test: $description${NC}"
    
    if timeout 5 ping -c 3 -W 1 $CONTAINER_IP >/dev/null 2>&1; then
        if [ "$should_work" = "yes" ]; then
            echo -e "${GREEN}✅ PASS: Ping works as expected${NC}"
            return 0
        else
            echo -e "${RED}❌ FAIL: Ping works but should be blocked!${NC}"
            return 1
        fi
    else
        if [ "$should_work" = "no" ]; then
            echo -e "${GREEN}✅ PASS: Ping blocked as expected${NC}"
            return 0
        else
            echo -e "${RED}❌ FAIL: Ping blocked but should work!${NC}"
            return 1
        fi
    fi
}

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- pkill -9 xdp-test 2>/dev/null || true"
    ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- ip link set dev eth0 xdp off 2>/dev/null || true"
    echo -e "${GREEN}Cleanup done${NC}"
}

trap cleanup EXIT

echo -e "${BLUE}═══ Phase 1: Baseline Test (No XDP) ═══${NC}"
cleanup
sleep 1
test_ping "Baseline connectivity (no XDP loaded)" "yes"

echo ""
echo -e "${BLUE}═══ Phase 2: Build & Deploy Latest Code ═══${NC}"
echo "Pulling latest code..."
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- bash -c 'cd /root/secbeat && git pull && source /root/.cargo/env && ./build_ebpf.sh 2>&1 | tail -5'"
echo -e "${GREEN}✅ eBPF program rebuilt${NC}"

echo ""
echo -e "${BLUE}═══ Phase 3: Load XDP (Should Block $MY_IP) ═══${NC}"

# Create test script that blocks our IP with correct byte order
ssh root@$PROXMOX_HOST "pct exec $CONTAINER_ID -- bash -c 'cat > /tmp/test_xdp.sh << '\''EOF'\''
#!/bin/bash
cd /root
source /root/.cargo/env

cargo new --bin xdp-quick-test 2>/dev/null || true
cd xdp-quick-test

cat > Cargo.toml << '\''TOML'\''
[package]
name = \"xdp-quick-test\"
version = \"0.1.0\"
edition = \"2021\"

[dependencies]
aya = { version = \"0.13\" }
anyhow = \"1.0\"
secbeat-common = { path = \"/root/secbeat/secbeat-common\", features = [\"user\"] }
TOML

cat > src/main.rs << '\''RUST'\''
use aya::{maps::HashMap as AyaHashMap, programs::{Xdp, XdpFlags}, Ebpf};
use std::net::Ipv4Addr;
use secbeat_common::BlockEntry;

fn main() -> Result<(), anyhow::Error> {
    let mut ebpf = Ebpf::load_file(\"/root/secbeat/target/bpf/secbeat-ebpf\")?;
    let program: &mut Xdp = ebpf.program_mut(\"secbeat_xdp\").unwrap().try_into()?;
    program.load()?;
    program.attach(\"eth0\", XdpFlags::default())?;
    
    let mut blocklist: AyaHashMap<_, u32, BlockEntry> = ebpf.take_map(\"BLOCKLIST\").unwrap().try_into()?;
    
    let ip = Ipv4Addr::new(192, 168, 100, 12);
    let key = u32::from_ne_bytes(ip.octets()); // NATIVE ENDIAN - THE FIX!
    
    blocklist.insert(key, BlockEntry { blocked_at: 0, hit_count: 0, flags: 0 }, 0)?;
    println!(\"Blocked {} (key: 0x{:08x})\", ip, key);
    std::thread::park();
    Ok(())
}
RUST

cargo build --release --quiet
nohup ./target/release/xdp-quick-test > /tmp/xdp.log 2>&1 &
sleep 2
cat /tmp/xdp.log
EOF
chmod +x /tmp/test_xdp.sh && /tmp/test_xdp.sh'"

echo -e "${GREEN}✅ XDP loaded with blocklist${NC}"
sleep 2

echo ""
echo -e "${BLUE}═══ Phase 4: Test Blocking ═══${NC}"
test_ping "After blocking $MY_IP" "no"

echo ""
echo -e "${BLUE}═══ Phase 5: Remove XDP & Test Recovery ═══${NC}"
cleanup
sleep 3  # Give more time for network stack to recover
test_ping "After removing XDP" "yes"

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   All Tests Completed Successfully! 🎉${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
