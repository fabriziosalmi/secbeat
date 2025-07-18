#!/bin/bash
# Script to prepare Ubuntu 24.04 cloud image template on Proxmox

set -e

# Read configuration from terraform.tfvars if available
if [[ -f terraform.tfvars ]]; then
    PROXMOX_HOST=$(grep "proxmox_host" terraform.tfvars | cut -d'"' -f2)
    STORAGE=$(grep "storage_pool" terraform.tfvars | cut -d'"' -f2)
else
    echo "Warning: terraform.tfvars not found, using default values"
    PROXMOX_HOST="192.168.100.2"
    STORAGE="local-lvm"
fi

TEMPLATE_ID="9000"
TEMPLATE_NAME="ubuntu-24.04-server"
IMAGE_URL="https://cloud-images.ubuntu.com/noble/current/noble-server-cloudimg-amd64.img"

echo "Setting up Ubuntu 24.04 cloud image template on Proxmox..."
echo "Host: $PROXMOX_HOST"
echo "Storage: $STORAGE"

# Download the cloud image if it doesn't exist
ssh root@${PROXMOX_HOST} "
    cd /var/lib/vz/template/iso
    if [ ! -f noble-server-cloudimg-amd64.img ]; then
        echo 'Downloading Ubuntu 24.04 cloud image...'
        wget -O noble-server-cloudimg-amd64.img ${IMAGE_URL}
    else
        echo 'Cloud image already exists'
    fi
"

# Check if template already exists
if ssh root@${PROXMOX_HOST} "qm status ${TEMPLATE_ID} >/dev/null 2>&1"; then
    echo "Template ${TEMPLATE_ID} already exists. Destroying it first..."
    ssh root@${PROXMOX_HOST} "qm destroy ${TEMPLATE_ID}"
fi

echo "Creating VM template ${TEMPLATE_ID}..."

# Create the template VM
ssh root@${PROXMOX_HOST} "
    # Create a new VM
    qm create ${TEMPLATE_ID} \
        --name ${TEMPLATE_NAME} \
        --memory 1024 \
        --cores 1 \
        --net0 virtio,bridge=vmbr0 \
        --scsihw virtio-scsi-pci \
        --scsi0 ${STORAGE}:0,import-from=/var/lib/vz/template/iso/noble-server-cloudimg-amd64.img,discard=on

    # Resize the disk to 20GB (minimum for template)
    qm resize ${TEMPLATE_ID} scsi0 20G

    # Add cloud-init drive
    qm set ${TEMPLATE_ID} --ide2 ${STORAGE}:cloudinit

    # Set boot disk
    qm set ${TEMPLATE_ID} --boot c --bootdisk scsi0

    # Enable serial console
    qm set ${TEMPLATE_ID} --serial0 socket --vga serial0

    # Enable agent
    qm set ${TEMPLATE_ID} --agent enabled=1

    # Convert to template
    qm template ${TEMPLATE_ID}
"

echo "Template ${TEMPLATE_NAME} (ID: ${TEMPLATE_ID}) created successfully!"
echo "You can now use this template with Terraform."
