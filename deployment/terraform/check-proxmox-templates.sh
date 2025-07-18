#!/bin/bash

# Check available VM templates in Proxmox
# This script uses the Proxmox API to list available templates

set -e

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

if [[ ! -f terraform.tfvars ]]; then
    print_error "terraform.tfvars not found"
    exit 1
fi

# Extract Proxmox connection details
PROXMOX_HOST=$(grep "proxmox_host" terraform.tfvars | cut -d'"' -f2)
PROXMOX_USER=$(grep "proxmox_user" terraform.tfvars | cut -d'"' -f2)
PROXMOX_PASSWORD=$(grep "proxmox_password" terraform.tfvars | cut -d'"' -f2)
TARGET_NODE=$(grep "target_node" terraform.tfvars | cut -d'"' -f2)

print_info "Checking available templates on Proxmox node: $TARGET_NODE"

# Use curl to check available VMs/templates via Proxmox API
# First, get a ticket
print_info "Getting Proxmox API ticket..."

TICKET_RESPONSE=$(curl -k -d "username=$PROXMOX_USER&password=$PROXMOX_PASSWORD" \
    "https://$PROXMOX_HOST:8006/api2/json/access/ticket" 2>/dev/null)

if [[ $? -ne 0 ]]; then
    print_error "Failed to connect to Proxmox API"
    exit 1
fi

# Extract ticket and CSRF token
TICKET=$(echo "$TICKET_RESPONSE" | grep -o '"ticket":"[^"]*"' | cut -d'"' -f4)
CSRF_TOKEN=$(echo "$TICKET_RESPONSE" | grep -o '"CSRFPreventionToken":"[^"]*"' | cut -d'"' -f4)

if [[ -z "$TICKET" ]]; then
    print_error "Failed to get Proxmox API ticket - check credentials"
    exit 1
fi

print_success "Successfully authenticated with Proxmox API"

# Get list of VMs/templates
print_info "Fetching VM/template list..."

VMS_RESPONSE=$(curl -k -b "PVEAuthCookie=$TICKET" \
    -H "CSRFPreventionToken: $CSRF_TOKEN" \
    "https://$PROXMOX_HOST:8006/api2/json/nodes/$TARGET_NODE/qemu" 2>/dev/null)

if [[ $? -ne 0 ]]; then
    print_error "Failed to fetch VM list from Proxmox API"
    exit 1
fi

print_info "Available VMs and Templates:"
echo "----------------------------------------"

# Parse and display VMs/templates
echo "$VMS_RESPONSE" | python3 -c "
import json
import sys

try:
    data = json.load(sys.stdin)
    if 'data' in data:
        vms = data['data']
        templates = [vm for vm in vms if vm.get('template', 0) == 1]
        regular_vms = [vm for vm in vms if vm.get('template', 0) != 1]
        
        if templates:
            print('📋 TEMPLATES:')
            for vm in templates:
                name = vm.get('name', 'N/A')
                vmid = vm.get('vmid', 'N/A')
                print(f'  - {name} (ID: {vmid})')
        else:
            print('⚠️  No templates found!')
            
        print()
        print('🖥️  REGULAR VMs:')
        for vm in regular_vms:
            name = vm.get('name', 'N/A')
            vmid = vm.get('vmid', 'N/A')
            status = vm.get('status', 'N/A')
            print(f'  - {name} (ID: {vmid}, Status: {status})')
    else:
        print('Error: No data in response')
        print(json.dumps(data, indent=2))
except Exception as e:
    print(f'Error parsing JSON: {e}')
    sys.exit(1)
"

echo "----------------------------------------"

# Check current template setting
CURRENT_TEMPLATE=$(grep "vm_template" terraform.tfvars | cut -d'"' -f2)
print_info "Current template configured: $CURRENT_TEMPLATE"

# Suggest common template names
echo
print_info "Common template naming patterns:"
echo "  - ubuntu-server-24.04"
echo "  - ubuntu-24.04"
echo "  - ubuntu-cloud"
echo "  - ubuntu-2404-cloudinit"
echo "  - debian-12"
echo "  - Use VMID instead of name (e.g., '9000')"

echo
print_info "To fix the deployment, update the 'vm_template' value in terraform.tfvars"
print_info "with one of the template names shown above."
