#!/bin/bash
# Quick test script for WASM engine on Proxmox

sshpass -p invaders ssh -o StrictHostKeyChecking=no root@192.168.100.102 \
  "pct exec 100 -- bash -c 'source /root/.cargo/env && cd /root/secbeat-wasm-test && ./target/release/wasm-test-runner'"
