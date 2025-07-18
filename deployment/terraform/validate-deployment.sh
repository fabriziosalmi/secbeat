#!/bin/bash

# SecBeat Deployment Validation Script
# Run this before deploying to catch potential issues

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE} SecBeat Deployment Validation${NC}"
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

# Global variables for validation
ERRORS=0
WARNINGS=0

validate_proxmox_connection() {
    print_info "Validating Proxmox connection..."
    
    local proxmox_host=$(grep "proxmox_host" terraform.tfvars | cut -d'"' -f2)
    local proxmox_user=$(grep "proxmox_user" terraform.tfvars | cut -d'"' -f2)
    
    # Test basic connectivity
    if ping -c 1 -W 3 "$proxmox_host" &>/dev/null; then
        print_success "Proxmox host $proxmox_host is reachable"
    else
        print_error "Cannot reach Proxmox host $proxmox_host"
        ((ERRORS++))
    fi
    
    # Test HTTPS port
    if timeout 5 bash -c "</dev/tcp/$proxmox_host/8006" 2>/dev/null; then
        print_success "Proxmox API port 8006 is accessible"
    else
        print_error "Cannot connect to Proxmox API on port 8006"
        ((ERRORS++))
    fi
}

validate_terraform_config() {
    print_info "Validating Terraform configuration..."
    
    # Check terraform syntax
    if terraform validate &>/dev/null; then
        print_success "Terraform configuration is valid"
    else
        print_error "Terraform configuration has syntax errors"
        terraform validate
        ((ERRORS++))
    fi
    
    # Check required variables
    local required_vars=("proxmox_host" "proxmox_user" "proxmox_password" "ssh_public_key" "vm_template" "storage_pool" "target_node")
    
    for var in "${required_vars[@]}"; do
        if grep -q "^$var" terraform.tfvars; then
            print_success "Variable '$var' is defined"
        else
            print_error "Required variable '$var' is missing from terraform.tfvars"
            ((ERRORS++))
        fi
    done
}

validate_ssh_key() {
    print_info "Validating SSH key..."
    
    local ssh_key=$(grep "ssh_public_key" terraform.tfvars | cut -d'"' -f2)
    
    if [[ -n "$ssh_key" && "$ssh_key" =~ ^ssh-(rsa|ed25519) ]]; then
        print_success "SSH public key format is valid"
        
        # Check if key is in the local SSH agent or file
        if ssh-add -L 2>/dev/null | grep -q "${ssh_key:0:50}"; then
            print_success "SSH key is loaded in SSH agent"
        elif [[ -f ~/.ssh/id_rsa.pub ]] && grep -q "${ssh_key:0:50}" ~/.ssh/id_rsa.pub; then
            print_success "SSH key matches local public key"
        else
            print_warning "SSH key not found locally - ensure you have the private key"
            ((WARNINGS++))
        fi
    else
        print_error "SSH public key format is invalid"
        ((ERRORS++))
    fi
}

validate_vm_template() {
    print_info "Validating VM template..."
    
    local vm_template=$(grep "vm_template" terraform.tfvars | cut -d'"' -f2)
    local proxmox_host=$(grep "proxmox_host" terraform.tfvars | cut -d'"' -f2)
    local proxmox_user=$(grep "proxmox_user" terraform.tfvars | cut -d'"' -f2)
    local proxmox_password=$(grep "proxmox_password" terraform.tfvars | cut -d'"' -f2)
    local target_node=$(grep "target_node" terraform.tfvars | cut -d'"' -f2)
    
    if command -v sshpass &>/dev/null; then
        print_info "Checking if template '$vm_template' exists in Proxmox..."
        
        # Check if template exists by looking for the template config flag
        local template_check=$(sshpass -p "$proxmox_password" ssh -o StrictHostKeyChecking=no "$proxmox_user@$proxmox_host" \
            "qm list | grep '$vm_template'" 2>/dev/null || echo "")
        
        # Also check if it's actually a template by looking at config
        if [[ -n "$template_check" ]]; then
            local vmid=$(echo "$template_check" | awk '{print $1}')
            local is_template=$(sshpass -p "$proxmox_password" ssh -o StrictHostKeyChecking=no "$proxmox_user@$proxmox_host" \
                "qm config $vmid | grep '^template:'" 2>/dev/null || echo "")
            
            if [[ -n "$is_template" ]]; then
                print_success "Template '$vm_template' found in Proxmox (ID: $vmid)"
                
                if [[ "$vm_template" == "ubuntu-24.04-server" || "$vm_template" == "ubuntu-22.04-server" ]]; then
                    print_success "Using recommended Ubuntu template: $vm_template"
                else
                    print_warning "Using non-standard template: $vm_template"
                    print_info "Ensure the template supports cloud-init and has required packages"
                    ((WARNINGS++))
                fi
            else
                print_error "VM '$vm_template' found but it's not a template"
                print_info "Convert to template: ssh root@$proxmox_host 'qm template $vmid'"
                ((ERRORS++))
            fi
        else
            print_error "Template '$vm_template' not found in Proxmox"
            print_info "Available options:"
            echo "  1. Create the template using: ./setup-template.sh"
            echo "  2. Update terraform.tfvars to use an existing template"
            echo "  3. List available templates: ./check-proxmox-templates.sh"
            ((ERRORS++))
        fi
    else
        print_warning "Cannot verify template existence (sshpass not available)"
        print_info "Template specified: $vm_template"
        if [[ "$vm_template" == "ubuntu-24.04-server" || "$vm_template" == "ubuntu-22.04-server" ]]; then
            print_info "Using recommended Ubuntu template: $vm_template"
        else
            print_warning "Using non-standard template: $vm_template"
            print_info "Ensure the template supports cloud-init and has required packages"
            ((WARNINGS++))
        fi
    fi
}

validate_network_ranges() {
    print_info "Validating network configuration..."
    
    # Check IP ranges don't conflict with protected LXCs
    local protected_ips=("192.168.100.236" "192.168.100.240")
    local secbeat_range_start=100
    local secbeat_range_end=109
    
    for protected_ip in "${protected_ips[@]}"; do
        local last_octet=$(echo "$protected_ip" | cut -d'.' -f4)
        if [[ $last_octet -ge $secbeat_range_start && $last_octet -le $secbeat_range_end ]]; then
            print_error "SecBeat IP range conflicts with protected LXC at $protected_ip"
            ((ERRORS++))
        else
            print_success "No IP conflict with protected LXC at $protected_ip"
        fi
    done
}

validate_memory_allocation() {
    print_info "Validating memory allocation..."
    
    # Calculate total memory needed
    local mitigation_mem=$((3 * 768))   # 3 nodes × 768MB
    local orchestrator_mem=768
    local nats_mem=$((3 * 512))         # 3 nodes × 512MB  
    local lb_mem=$((2 * 768))           # 2 nodes × 768MB
    local monitoring_mem=1536
    
    local total_secbeat_mem=$((mitigation_mem + orchestrator_mem + nats_mem + lb_mem + monitoring_mem))
    local existing_lxc_mem=3072         # 2GB + 1GB
    local host_overhead=1536            # 1.5GB
    local total_used=$((total_secbeat_mem + existing_lxc_mem + host_overhead))
    local system_total=16384            # 16GB
    local free_mem=$((system_total - total_used))
    local free_percent=$((free_mem * 100 / system_total))
    
    print_info "Memory allocation analysis:"
    echo "  - SecBeat VMs: ${total_secbeat_mem}MB (7.5GB)"
    echo "  - Existing LXCs: ${existing_lxc_mem}MB (3GB)"
    echo "  - Host overhead: ${host_overhead}MB (1.5GB)"
    echo "  - Total used: ${total_used}MB (12GB)"
    echo "  - Free memory: ${free_mem}MB (4GB)"
    echo "  - Free percentage: ${free_percent}%"
    
    if [[ $free_percent -ge 20 ]]; then
        print_success "Memory allocation is safe (${free_percent}% free)"
    elif [[ $free_percent -ge 10 ]]; then
        print_warning "Memory allocation is tight (${free_percent}% free)"
        ((WARNINGS++))
    else
        print_error "Insufficient memory (only ${free_percent}% free)"
        ((ERRORS++))
    fi
}

validate_storage() {
    print_info "Validating storage requirements..."
    
    local total_disk=$((3*8 + 8 + 3*6 + 2*6 + 12))  # Total GB needed
    
    print_info "Storage requirements:"
    echo "  - Mitigation nodes: $((3*8))GB (3 × 8GB)"
    echo "  - Orchestrator: 8GB"
    echo "  - NATS cluster: $((3*6))GB (3 × 6GB)"
    echo "  - Load balancers: $((2*6))GB (2 × 6GB)"
    echo "  - Monitoring: 12GB"
    echo "  - Total required: ${total_disk}GB"
    
    print_success "Storage allocation configured for ${total_disk}GB"
}

check_terraform_plan() {
    print_info "Checking Terraform execution plan..."
    
    # Capture terraform plan exit code
    terraform plan -detailed-exitcode &>/dev/null
    local exit_code=$?
    
    if [[ $exit_code -eq 0 ]]; then
        print_warning "No changes planned - infrastructure already exists"
        ((WARNINGS++))
    elif [[ $exit_code -eq 2 ]]; then
        print_success "Terraform plan shows changes ready to apply"
    elif [[ $exit_code -eq 1 ]]; then
        print_error "Terraform plan failed"
        print_info "Running terraform plan to show errors:"
        terraform plan
        ((ERRORS++))
    else
        print_error "Terraform plan failed with unexpected exit code: $exit_code"
        ((ERRORS++))
    fi
}

run_all_validations() {
    print_header
    
    # Pre-checks
    if [[ ! -f terraform.tfvars ]]; then
        print_error "terraform.tfvars file not found"
        exit 1
    fi
    
    if ! command -v terraform &>/dev/null; then
        print_error "Terraform not installed"
        exit 1
    fi
    
    if ! command -v sshpass &>/dev/null; then
        print_warning "sshpass not installed - template validation will be skipped"
        print_info "Install sshpass: brew install sshpass (macOS) or apt-get install sshpass (Ubuntu)"
        ((WARNINGS++))
    fi
    
    # Run validations
    validate_terraform_config
    validate_proxmox_connection
    validate_ssh_key
    validate_vm_template
    validate_network_ranges
    validate_memory_allocation
    validate_storage
    check_terraform_plan
    
    # Summary
    echo
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE} Validation Summary${NC}"
    echo -e "${BLUE}================================${NC}"
    
    if [[ $ERRORS -eq 0 && $WARNINGS -eq 0 ]]; then
        print_success "All validations passed! Ready to deploy."
        echo
        print_info "To deploy SecBeat infrastructure:"
        echo "  terraform apply"
        exit 0
    elif [[ $ERRORS -eq 0 ]]; then
        print_warning "$WARNINGS warning(s) found, but deployment should work"
        echo
        print_info "To deploy SecBeat infrastructure:"
        echo "  terraform apply"
        exit 0
    else
        print_error "$ERRORS error(s) and $WARNINGS warning(s) found"
        echo
        print_error "Fix the errors before deploying!"
        exit 1
    fi
}

# Run if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    run_all_validations
fi
