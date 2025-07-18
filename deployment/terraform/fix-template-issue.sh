#!/bin/bash

# Quick fix for missing VM template issue
# This script will help you resolve the "vm 'ubuntu-24.04-server' not found" error

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE} SecBeat Template Fix${NC}"
    echo -e "${BLUE}================================${NC}"
    echo
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

print_header

print_error "Terraform deployment failed because the VM template 'ubuntu-24.04-server' was not found."
echo

print_info "Here are your options to fix this:"
echo

echo "Option 1: Create the required template (RECOMMENDED)"
echo "  This will create the ubuntu-24.04-server template in your Proxmox:"
echo "  ./setup-template.sh"
echo

echo "Option 2: Use an existing template"
echo "  First, check what templates are available:"
echo "  ./check-proxmox-templates.sh"
echo "  Then update terraform.tfvars with an existing template name."
echo

echo "Option 3: Use a different VM template name"
echo "  If you have a different Ubuntu template, update terraform.tfvars:"
echo "  vm_template = \"your-existing-template-name\""
echo

print_warning "The deployment was partially completed (certificates were created)."
print_info "You can safely run 'terraform apply' again after fixing the template issue."
echo

print_info "Current Terraform state:"
terraform state list 2>/dev/null || echo "No Terraform state found"

echo
print_info "To clean up the partial deployment (if needed):"
echo "  terraform destroy"
echo

print_info "After fixing the template, run the validation again:"
echo "  ./validate-deployment.sh"
