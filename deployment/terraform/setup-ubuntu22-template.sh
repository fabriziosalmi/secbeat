#!/bin/bash
# Script to create Ubuntu 22.04 template from existing ISO

set -e

# Read configuration from terraform.tfvars
if [[ -f terraform.tfvars ]]; then
    PROXMOX_HOST=$(grep "proxmox_host" terraform.tfvars | cut -d'"' -f2)
    PROXMOX_USER=$(grep "proxmox_user" terraform.tfvars | cut -d'"' -f2)
    PROXMOX_PASSWORD=$(grep "proxmox_password" terraform.tfvars | cut -d'"' -f2)
    STORAGE=$(grep "storage_pool" terraform.tfvars | cut -d'"' -f2)
    TARGET_NODE=$(grep "target_node" terraform.tfvars | cut -d'"' -f2)
else
    echo "Error: terraform.tfvars not found"
    exit 1
fi

TEMPLATE_ID="9001"
TEMPLATE_NAME="ubuntu-22.04-server"
ISO_NAME="ubuntu-22.04.5-live-server-amd64.iso"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
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

echo "================================"
echo " Ubuntu 22.04 Template Setup"
echo "================================"
echo
print_info "Creating Ubuntu 22.04 template from ISO..."
print_info "Host: $PROXMOX_HOST"
print_info "Node: $TARGET_NODE"
print_info "Storage: $STORAGE"
print_info "Template ID: $TEMPLATE_ID"
echo

# Check if template already exists
if sshpass -p "$PROXMOX_PASSWORD" ssh -o StrictHostKeyChecking=no "$PROXMOX_USER@$PROXMOX_HOST" "qm status $TEMPLATE_ID >/dev/null 2>&1"; then
    print_warning "Template $TEMPLATE_ID already exists. Destroying it first..."
    sshpass -p "$PROXMOX_PASSWORD" ssh -o StrictHostKeyChecking=no "$PROXMOX_USER@$PROXMOX_HOST" "qm destroy $TEMPLATE_ID"
fi

# Check if ISO exists
if ! sshpass -p "$PROXMOX_PASSWORD" ssh -o StrictHostKeyChecking=no "$PROXMOX_USER@$PROXMOX_HOST" "ls /var/lib/vz/template/iso/$ISO_NAME >/dev/null 2>&1"; then
    print_error "ISO file $ISO_NAME not found in /var/lib/vz/template/iso/"
    print_info "Please upload the Ubuntu 22.04.5 ISO to Proxmox first"
    exit 1
fi

print_success "ISO file found: $ISO_NAME"

print_info "Creating VM template $TEMPLATE_ID..."

# Create the template VM
sshpass -p "$PROXMOX_PASSWORD" ssh -o StrictHostKeyChecking=no "$PROXMOX_USER@$PROXMOX_HOST" "
    # Create a new VM
    qm create $TEMPLATE_ID \\
        --name $TEMPLATE_NAME \\
        --memory 2048 \\
        --cores 2 \\
        --net0 virtio,bridge=vmbr0 \\
        --scsihw virtio-scsi-pci \\
        --scsi0 $STORAGE:20,discard=on \\
        --ide2 $STORAGE:cloudinit \\
        --boot c --bootdisk scsi0 \\
        --serial0 socket --vga serial0 \\
        --agent enabled=1 \\
        --ostype l26

    # Set the ISO as CD-ROM
    qm set $TEMPLATE_ID --cdrom local:iso/$ISO_NAME

    echo 'VM $TEMPLATE_ID created successfully'
"

print_success "VM $TEMPLATE_ID created with Ubuntu 22.04 ISO"
print_warning "MANUAL STEPS REQUIRED:"
echo
echo "1. Start the VM and install Ubuntu 22.04:"
echo "   - Go to Proxmox web interface"
echo "   - Start VM $TEMPLATE_ID"
echo "   - Install Ubuntu with these settings:"
echo "     * Enable OpenSSH server"
echo "     * Create user 'ubuntu' with your SSH key"
echo "     * Install cloud-init: apt install cloud-init"
echo
echo "2. After installation, prepare for template:"
echo "   - Clean the system: sudo apt clean && sudo rm -rf /var/log/* /tmp/*"
echo "   - Shut down the VM"
echo
echo "3. Convert to template:"
echo "   ssh root@$PROXMOX_HOST 'qm template $TEMPLATE_ID'"
echo
print_info "Alternative: Use the cloud image script for automated setup:"
echo "   ./setup-ubuntu22-cloud-template.sh"
