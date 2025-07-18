#!/bin/bash

# SecBeat Kernel Permissions Setup Script
# Run this after installing SecBeat binaries

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    print_error "This script should not be run as root"
    exit 1
fi

# Check if binaries exist
MITIGATION_BIN="/usr/local/bin/mitigation-node"
ORCHESTRATOR_BIN="/usr/local/bin/orchestrator-node"

if [[ ! -f "$MITIGATION_BIN" ]]; then
    print_error "Mitigation binary not found at $MITIGATION_BIN"
    print_info "Please install SecBeat binaries first"
    exit 1
fi

print_info "Setting up kernel-level permissions for SecBeat..."

# Set capabilities on mitigation binary
print_info "Setting capabilities on mitigation-node binary..."
sudo setcap cap_net_raw,cap_net_admin+ep "$MITIGATION_BIN"

# Verify capabilities
if getcap "$MITIGATION_BIN" | grep -q "cap_net_admin,cap_net_raw+ep"; then
    print_success "Capabilities set successfully on mitigation-node"
else
    print_error "Failed to set capabilities on mitigation-node"
    exit 1
fi

# Check if secbeat user exists
if ! id secbeat &>/dev/null; then
    print_warning "secbeat user not found, creating..."
    sudo useradd -r -s /bin/bash -d /var/lib/secbeat -m secbeat
    sudo usermod -aG sudo secbeat
fi

# Add secbeat user to netdev group
print_info "Adding secbeat user to netdev group..."
sudo usermod -aG netdev secbeat

# Create sudoers file for capability management
print_info "Setting up sudoers for capability management..."
echo "secbeat ALL=(ALL) NOPASSWD: /sbin/setcap" | sudo tee /etc/sudoers.d/secbeat-caps
sudo chmod 440 /etc/sudoers.d/secbeat-caps

# Create necessary directories
print_info "Creating SecBeat directories..."
sudo mkdir -p /opt/secbeat/{bin,config,logs,data}
sudo mkdir -p /var/lib/secbeat
sudo mkdir -p /var/log/secbeat
sudo mkdir -p /etc/secbeat

# Set ownership
sudo chown -R secbeat:secbeat /opt/secbeat
sudo chown -R secbeat:secbeat /var/lib/secbeat
sudo chown -R secbeat:secbeat /var/log/secbeat
sudo chown -R secbeat:secbeat /etc/secbeat

# Copy systemd service files
print_info "Installing systemd service files..."
if [[ -f "systemd/secbeat-mitigation.service" ]]; then
    sudo cp systemd/secbeat-mitigation.service /etc/systemd/system/
    print_success "Installed secbeat-mitigation.service"
else
    print_warning "systemd/secbeat-mitigation.service not found"
fi

if [[ -f "systemd/secbeat-orchestrator.service" ]]; then
    sudo cp systemd/secbeat-orchestrator.service /etc/systemd/system/
    print_success "Installed secbeat-orchestrator.service"
else
    print_warning "systemd/secbeat-orchestrator.service not found"
fi

if [[ -f "systemd/nats.service" ]]; then
    sudo cp systemd/nats.service /etc/systemd/system/
    print_success "Installed nats.service"
else
    print_warning "systemd/nats.service not found"
fi

# Reload systemd
print_info "Reloading systemd daemon..."
sudo systemctl daemon-reload

# Apply kernel parameters
print_info "Applying kernel parameters..."
if [[ -f "/etc/sysctl.d/99-secbeat.conf" ]]; then
    sudo sysctl -p /etc/sysctl.d/99-secbeat.conf
    print_success "Applied kernel parameters"
else
    print_warning "Kernel parameters file not found, creating basic version..."
    cat << 'EOF' | sudo tee /etc/sysctl.d/99-secbeat.conf
# SecBeat kernel parameters
net.ipv4.ip_forward = 1
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.default.send_redirects = 0
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.core.netdev_max_backlog = 5000
net.core.somaxconn = 1024
net.ipv4.tcp_max_syn_backlog = 8192
kernel.unprivileged_userns_clone = 1
EOF
    sudo sysctl -p /etc/sysctl.d/99-secbeat.conf
fi

# Test capabilities
print_info "Testing kernel access..."
if sudo -u secbeat timeout 5 "$MITIGATION_BIN" --test-caps 2>/dev/null; then
    print_success "Kernel access test passed"
else
    print_warning "Kernel access test failed or binary doesn't support --test-caps flag"
fi

# Show status
print_info "Kernel permissions setup complete!"
echo
print_info "Next steps:"
echo "1. Configure SecBeat: /etc/secbeat/mitigation.toml"
echo "2. Enable services: sudo systemctl enable --now secbeat-mitigation"
echo "3. Check status: sudo systemctl status secbeat-mitigation"
echo "4. View logs: journalctl -u secbeat-mitigation -f"
echo
print_info "For troubleshooting, see: KERNEL_OPERATIONS.md"
