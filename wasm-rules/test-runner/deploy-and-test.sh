#!/bin/bash
# Deploy and test WASM engine on Proxmox container 100
# Proxmox: 192.168.100.102 (root/invaders)
# Container: vmid 100 (Ubuntu)

set -e

PROXMOX_HOST="192.168.100.102"
PROXMOX_USER="root"
PROXMOX_PASS="invaders"
CONTAINER_ID="100"

echo "🚀 SecBeat WASM Deployment to Proxmox Container $CONTAINER_ID"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 1: Build WASM module locally
echo ""
echo "Step 1: Building WASM module..."
cd ../bad-bot
./build.sh
cd ../test-runner

# Step 2: Create deployment package
echo ""
echo "Step 2: Creating deployment package..."
mkdir -p /tmp/secbeat-wasm-test
cp -r Cargo.toml src /tmp/secbeat-wasm-test/
cp ../../target/wasm/bad-bot.wasm /tmp/secbeat-wasm-test/
cd /tmp

# Step 3: Create tarball
echo ""
echo "Step 3: Creating tarball..."
tar czf secbeat-wasm-test.tar.gz secbeat-wasm-test/

# Step 4: Transfer to Proxmox host
echo ""
echo "Step 4: Transferring files to Proxmox host..."
sshpass -p "$PROXMOX_PASS" scp -o StrictHostKeyChecking=no \
    secbeat-wasm-test.tar.gz \
    ${PROXMOX_USER}@${PROXMOX_HOST}:/tmp/

# Step 5: Deploy to container, compile, and run
echo ""
echo "Step 5: Deploying to container $CONTAINER_ID..."
sshpass -p "$PROXMOX_PASS" ssh -o StrictHostKeyChecking=no \
    ${PROXMOX_USER}@${PROXMOX_HOST} << 'ENDSSH'

# Copy tarball into container
pct push 100 /tmp/secbeat-wasm-test.tar.gz /root/secbeat-wasm-test.tar.gz

# Extract and build inside container
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Building and running tests in container..."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

pct exec 100 -- bash -c "
source /root/.cargo/env
cd /root
tar xzf secbeat-wasm-test.tar.gz
cd secbeat-wasm-test
echo 'Building test runner in container...'
cargo build --release 2>&1 | tail -10
echo ''
echo 'Running tests...'
./target/release/wasm-test-runner
"

ENDSSH

echo ""
echo "✓ Deployment and testing complete!"
