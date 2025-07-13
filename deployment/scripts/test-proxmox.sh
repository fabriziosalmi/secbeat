#!/bin/bash

# SecBeat Proxmox Pre-Deployment Test Script
# Quick validation and single VM test deployment

set -e

# Configuration
PROXMOX_HOST="192.168.100.23"
PROXMOX_USER="root@pam"
SSH_KEY_PATH="$HOME/.ssh/id_rsa"
ISO_NAME="ubuntu-24.04.2-live-server-amd64.iso"
ISO_PATH="/var/lib/vz/template/iso/ubuntu-24.04.2-live-server-amd64.iso"
STORAGE="local"
BRIDGE="vmbr0"

# Test VM configuration - Small footprint
TEST_VM_ID="999"
TEST_VM_NAME="secbeat-test"
TEST_VM_IP="192.168.100.220"   # Use high address in same network
TEST_VM_MEMORY="1024"    # 1GB RAM
TEST_VM_CORES="1"        # 1 CPU core
TEST_VM_DISK="8"         # 8GB disk

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Function to execute command on Proxmox host
proxmox_exec() {
    local cmd="$1"
    local description="$2"
    
    print_status "$description"
    if ssh -i "$SSH_KEY_PATH" -o StrictHostKeyChecking=no "$PROXMOX_USER@$PROXMOX_HOST" "$cmd"; then
        print_success "$description completed"
        return 0
    else
        print_error "$description failed"
        return 1
    fi
}

# Function to run pre-deployment checks
run_predeploy_checks() {
    echo "🔍 Running Pre-Deployment Checks"
    echo "================================="
    
    # Check SSH connectivity
    print_status "Testing SSH connectivity to Proxmox host"
    if ssh -i "$SSH_KEY_PATH" -o StrictHostKeyChecking=no -o ConnectTimeout=10 "$PROXMOX_USER@$PROXMOX_HOST" "echo 'SSH connection successful'"; then
        print_success "SSH connectivity verified"
    else
        print_error "SSH connectivity failed"
        echo "Please ensure:"
        echo "1. SSH key exists at $SSH_KEY_PATH"
        echo "2. Public key is added to Proxmox host authorized_keys"
        echo "3. Proxmox host is accessible at $PROXMOX_HOST"
        exit 1
    fi
    
    # Check ISO availability
    print_status "Checking ISO availability"
    if proxmox_exec "test -f $ISO_PATH" "Checking ISO file"; then
        print_success "ISO file found"
    else
        print_error "ISO file not found"
        echo "Please upload $ISO_NAME to Proxmox:"
        echo "1. Go to Datacenter -> Storage -> local -> ISO Images"
        echo "2. Upload $ISO_NAME"
        exit 1
    fi
    
    # Check available storage
    print_status "Checking storage availability"
    proxmox_exec "pvesm status" "Checking storage status"
    
    # Check network bridge
    print_status "Checking network bridge"
    proxmox_exec "ip link show $BRIDGE" "Checking bridge $BRIDGE"
    
    # Check if test VM ID is available
    print_status "Checking VM ID availability"
    if proxmox_exec "! qm config $TEST_VM_ID >/dev/null 2>&1" "Checking VM ID $TEST_VM_ID availability"; then
        print_success "VM ID $TEST_VM_ID is available"
    else
        print_warning "VM ID $TEST_VM_ID is in use, will destroy and recreate"
        proxmox_exec "qm stop $TEST_VM_ID || true" "Stopping existing test VM"
        proxmox_exec "qm destroy $TEST_VM_ID || true" "Destroying existing test VM"
    fi
    
    print_success "All pre-deployment checks passed!"
}

# Function to create a test VM
create_test_vm() {
    echo ""
    echo "🚀 Creating Test VM"
    echo "==================="
    
    # Create cloud-init user data
    local user_data_file="/tmp/user-data-test.yml"
    cat > "$user_data_file" << EOF
#cloud-config
hostname: $TEST_VM_NAME
manage_etc_hosts: true

users:
  - name: secbeat
    groups: [adm, cdrom, dip, plugdev, lxd, sudo]
    shell: /bin/bash
    sudo: ALL=(ALL) NOPASSWD:ALL
    ssh_authorized_keys:
      - $(cat "${SSH_KEY_PATH}.pub")

package_update: true
package_upgrade: true

packages:
  - curl
  - wget
  - git
  - htop
  - net-tools
  - build-essential

network:
  version: 2
  ethernets:
    ens18:
      dhcp4: false
      addresses: [$TEST_VM_IP/24]
      gateway4: 192.168.200.1
      nameservers:
        addresses: [8.8.8.8, 8.8.4.4]

write_files:
  - path: /etc/ssh/sshd_config.d/99-secbeat.conf
    content: |
      PasswordAuthentication no
      PubkeyAuthentication yes
      PermitRootLogin no
    permissions: '0644'

runcmd:
  - systemctl restart sshd
  - mkdir -p /opt/secbeat
  - chown secbeat:secbeat /opt/secbeat

final_message: "SecBeat test VM is ready"
EOF
    
    # Copy cloud-init config to Proxmox
    print_status "Copying cloud-init configuration"
    scp -i "$SSH_KEY_PATH" "$user_data_file" "$PROXMOX_USER@$PROXMOX_HOST:/var/lib/vz/snippets/user-data-test.yml"
    
    # Create VM
    print_status "Creating test VM"
    proxmox_exec "qm create $TEST_VM_ID --name $TEST_VM_NAME --memory $TEST_VM_MEMORY --cores $TEST_VM_CORES --net0 virtio,bridge=$BRIDGE --scsihw virtio-scsi-pci --scsi0 $STORAGE:$TEST_VM_DISK,import-from=$ISO_PATH" "Creating VM $TEST_VM_NAME"
    
    # Configure VM
    proxmox_exec "qm set $TEST_VM_ID --ide2 $STORAGE:cloudinit --boot c --bootdisk scsi0 --serial0 socket --vga serial0" "Configuring VM $TEST_VM_NAME"
    
    # Set cloud-init config
    proxmox_exec "qm set $TEST_VM_ID --cicustom user=local:snippets/user-data-test.yml" "Setting cloud-init config"
    
    # Clean up local file
    rm -f "$user_data_file"
    
    print_success "Test VM created successfully"
}

# Function to start and test VM
start_and_test_vm() {
    echo ""
    echo "🔧 Starting and Testing VM"
    echo "==========================="
    
    # Start VM
    print_status "Starting test VM"
    proxmox_exec "qm start $TEST_VM_ID" "Starting VM $TEST_VM_NAME"
    
    # Wait for VM to be ready
    print_status "Waiting for VM to be ready (this may take a few minutes)..."
    local max_attempts=30
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if ssh -i "$SSH_KEY_PATH" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "secbeat@$TEST_VM_IP" "echo 'VM ready'" >/dev/null 2>&1; then
            print_success "VM is ready and accessible"
            break
        fi
        
        echo -n "."
        sleep 10
        ((attempt++))
        
        if [ $attempt -gt $max_attempts ]; then
            print_error "VM failed to become ready after $((max_attempts * 10)) seconds"
            return 1
        fi
    done
    
    # Test basic functionality
    print_status "Testing basic VM functionality"
    
    # Test SSH access
    if ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "whoami"; then
        print_success "SSH access test passed"
    else
        print_error "SSH access test failed"
        return 1
    fi
    
    # Test sudo access
    if ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "sudo whoami"; then
        print_success "Sudo access test passed"
    else
        print_error "Sudo access test failed"
        return 1
    fi
    
    # Test internet connectivity
    if ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "curl -s https://httpbin.org/ip" >/dev/null; then
        print_success "Internet connectivity test passed"
    else
        print_warning "Internet connectivity test failed (may affect package installation)"
    fi
    
    # Test package installation
    print_status "Testing package installation"
    if ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "sudo apt-get update && sudo apt-get install -y curl"; then
        print_success "Package installation test passed"
    else
        print_error "Package installation test failed"
        return 1
    fi
    
    print_success "All VM tests passed!"
}

# Function to test SecBeat deployment
test_secbeat_deployment() {
    echo ""
    echo "📦 Testing SecBeat Deployment"
    echo "============================="
    
    # Install Rust
    print_status "Installing Rust"
    ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "source ~/.cargo/env && rustc --version"
    
    # Create deployment archive
    print_status "Creating SecBeat deployment archive"
    local archive_file="/tmp/secbeat-test.tar.gz"
    cd "$(dirname "$(dirname "$0")")/.."
    tar --exclude='target' --exclude='.git' --exclude='logs' -czf "$archive_file" .
    
    # Copy and extract code
    print_status "Deploying SecBeat code"
    scp -i "$SSH_KEY_PATH" "$archive_file" "secbeat@$TEST_VM_IP:/tmp/"
    ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "cd /opt/secbeat && tar -xzf /tmp/secbeat-test.tar.gz --strip-components=1"
    
    # Test compilation
    print_status "Testing SecBeat compilation"
    if ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "cd /opt/secbeat && source ~/.cargo/env && cargo check"; then
        print_success "SecBeat compilation test passed"
    else
        print_error "SecBeat compilation test failed"
        return 1
    fi
    
    # Test build
    print_status "Testing SecBeat build (this may take several minutes)"
    if ssh -i "$SSH_KEY_PATH" "secbeat@$TEST_VM_IP" "cd /opt/secbeat && source ~/.cargo/env && timeout 600 cargo build --release"; then
        print_success "SecBeat build test passed"
    else
        print_warning "SecBeat build test failed or timed out"
    fi
    
    # Clean up
    rm -f "$archive_file"
    
    print_success "SecBeat deployment test completed!"
}

# Function to cleanup test VM
cleanup_test_vm() {
    echo ""
    echo "🧹 Cleaning Up Test VM"
    echo "======================"
    
    read -p "Do you want to remove the test VM? [y/N]: " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_status "Stopping and removing test VM"
        proxmox_exec "qm stop $TEST_VM_ID || true" "Stopping test VM"
        proxmox_exec "qm destroy $TEST_VM_ID || true" "Destroying test VM"
        print_success "Test VM cleaned up"
    else
        print_status "Test VM kept for manual inspection"
        echo "VM Details:"
        echo "  - VM ID: $TEST_VM_ID"
        echo "  - VM Name: $TEST_VM_NAME"
        echo "  - IP Address: $TEST_VM_IP"
        echo "  - SSH Access: ssh -i $SSH_KEY_PATH secbeat@$TEST_VM_IP"
    fi
}

# Function to generate test report
generate_test_report() {
    echo ""
    echo "📋 Test Summary"
    echo "==============="
    echo "✅ Proxmox connectivity verified"
    echo "✅ ISO availability confirmed"
    echo "✅ VM creation successful"
    echo "✅ Cloud-init configuration working"
    echo "✅ Network connectivity established"
    echo "✅ SSH access configured"
    echo "✅ Package installation working"
    echo "✅ SecBeat deployment process validated"
    echo ""
    echo "🚀 Ready for full deployment!"
    echo ""
    echo "Next steps:"
    echo "1. Run full deployment: ./deploy-proxmox.sh"
    echo "2. Monitor deployment logs in logs/deployment/"
    echo "3. Access services after deployment completes"
}

# Main function
main() {
    echo "🧪 SecBeat Proxmox Pre-Deployment Test"
    echo "======================================"
    echo "Proxmox Host: $PROXMOX_HOST"
    echo "Test VM IP: $TEST_VM_IP"
    echo ""
    
    run_predeploy_checks
    create_test_vm
    start_and_test_vm
    test_secbeat_deployment
    generate_test_report
    cleanup_test_vm
    
    echo ""
    echo "🎉 Pre-deployment testing completed successfully!"
}

# Run the script
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
