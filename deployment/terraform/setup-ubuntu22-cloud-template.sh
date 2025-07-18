#!/bin/bash
# Script to create Ubuntu 22.04 cloud template (automated)

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
IMAGE_URL="https://cloud-images.ubuntu.com/jammy/current/jammy-server-cloudimg-amd64.img"

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

echo "======================================="
echo " Ubuntu 22.04 Cloud Template Setup"
echo "======================================="
echo
print_info "Creating Ubuntu 22.04 cloud template..."
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

print_info "Downloading and setting up cloud image..."

# Download the cloud image and create template
sshpass -p "$PROXMOX_PASSWORD" ssh -o StrictHostKeyChecking=no "$PROXMOX_USER@$PROXMOX_HOST" "
    cd /var/lib/vz/template/iso
    
    # Download Ubuntu 22.04 cloud image if not exists
    if [ ! -f jammy-server-cloudimg-amd64.img ]; then
        echo 'Downloading Ubuntu 22.04 cloud image...'
        wget -O jammy-server-cloudimg-amd64.img $IMAGE_URL
    else
        echo 'Cloud image already exists'
    fi

    # Create a new VM
    qm create $TEMPLATE_ID \\
        --name $TEMPLATE_NAME \\
        --memory 1024 \\
        --cores 1 \\
        --net0 virtio,bridge=vmbr0 \\
        --scsihw virtio-scsi-pci \\
        --scsi0 $STORAGE:0,import-from=/var/lib/vz/template/iso/jammy-server-cloudimg-amd64.img,discard=on

    # Resize the disk to 20GB
    qm resize $TEMPLATE_ID scsi0 20G

    # Add cloud-init drive
    qm set $TEMPLATE_ID --ide2 $STORAGE:cloudinit

    # Set boot disk
    qm set $TEMPLATE_ID --boot c --bootdisk scsi0

    # Enable serial console
    qm set $TEMPLATE_ID --serial0 socket --vga serial0

    # Enable agent
    qm set $TEMPLATE_ID --agent enabled=1

    # Convert to template
    qm template $TEMPLATE_ID

    echo 'Template $TEMPLATE_NAME created successfully!'
"

print_success "Template $TEMPLATE_NAME (ID: $TEMPLATE_ID) created successfully!"
print_success "You can now use this template with Terraform."
echo
print_info "The template is ready to use with cloud-init and supports:"
echo "  - Automatic SSH key injection"
echo "  - Network configuration"
echo "  - Hostname setting"
echo "  - User creation"
