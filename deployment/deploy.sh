#!/bin/bash

# SecBeat Terraform + Ansible Deployment Script
# Complete infrastructure provisioning and configuration automation

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
TERRAFORM_DIR="$SCRIPT_DIR/terraform"
ANSIBLE_DIR="$SCRIPT_DIR/ansible"
LOG_DIR="$PROJECT_ROOT/logs/deployment"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

# Create log directory
mkdir -p "$LOG_DIR"
DEPLOY_LOG="$LOG_DIR/terraform_ansible_$(date +%Y%m%d_%H%M%S).log"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$DEPLOY_LOG"
}

# Print functions
print_header() {
    echo -e "\n${PURPLE}================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}================================${NC}\n"
    log "HEADER: $1"
}

print_status() {
    echo -e "${CYAN}⏳ $1${NC}"
    log "STATUS: $1"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    log "SUCCESS: $1"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
    log "ERROR: $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    log "WARNING: $1"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
    log "INFO: $1"
}

# Function to show usage
show_usage() {
    cat << 'EOF'
SecBeat Terraform + Ansible Deployment

USAGE:
    ./deploy.sh [COMMAND] [OPTIONS]

COMMANDS:
    init          Initialize Terraform and check prerequisites
    plan          Show Terraform execution plan
    apply         Deploy infrastructure with Terraform
    configure     Run Ansible configuration
    deploy        Complete deployment (apply + configure)
    test          Run deployment tests
    destroy       Destroy all infrastructure
    status        Check deployment status
    help          Show this help message

OPTIONS:
    -v, --verbose       Verbose output
    -f, --force         Force operation without confirmation
    --skip-terraform    Skip Terraform operations (Ansible only)
    --skip-ansible      Skip Ansible operations (Terraform only)

EXAMPLES:
    ./deploy.sh init                # Initialize and check prerequisites
    ./deploy.sh plan                # Show what will be created
    ./deploy.sh deploy              # Full deployment
    ./deploy.sh configure           # Configure existing VMs with Ansible
    ./deploy.sh test                # Test deployment
    ./deploy.sh destroy             # Destroy everything

PREREQUISITES:
    - Terraform >= 1.0
    - Ansible >= 2.9
    - SSH key at ~/.ssh/id_rsa
    - Proxmox access configured
    - terraform.tfvars file created

EOF
}

# Function to check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"
    
    # Check Terraform
    if command -v terraform >/dev/null 2>&1; then
        local tf_version=$(terraform version -json | jq -r '.terraform_version')
        print_success "Terraform found: $tf_version"
    else
        print_error "Terraform not found. Please install Terraform >= 1.0"
        return 1
    fi
    
    # Check Ansible
    if command -v ansible >/dev/null 2>&1; then
        local ansible_version=$(ansible --version | head -n1 | cut -d' ' -f2)
        print_success "Ansible found: $ansible_version"
    else
        print_error "Ansible not found. Please install Ansible >= 2.9"
        return 1
    fi
    
    # Check SSH key
    if [[ -f "$HOME/.ssh/id_rsa" ]]; then
        print_success "SSH key found at ~/.ssh/id_rsa"
    else
        print_error "SSH key not found at ~/.ssh/id_rsa"
        print_info "Generate SSH key with: ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa"
        return 1
    fi
    
    # Check terraform.tfvars
    if [[ -f "$TERRAFORM_DIR/terraform.tfvars" ]]; then
        print_success "Terraform variables file found"
    else
        print_error "terraform.tfvars not found"
        print_info "Copy terraform.tfvars.example to terraform.tfvars and configure it"
        return 1
    fi
    
    # Check jq
    if command -v jq >/dev/null 2>&1; then
        print_success "jq found"
    else
        print_error "jq not found. Please install jq for JSON processing"
        return 1
    fi
    
    print_success "All prerequisites met"
}

# Function to initialize Terraform
terraform_init() {
    print_header "Initializing Terraform"
    
    cd "$TERRAFORM_DIR"
    
    print_status "Initializing Terraform providers..."
    if terraform init; then
        print_success "Terraform initialized successfully"
    else
        print_error "Terraform initialization failed"
        return 1
    fi
    
    print_status "Validating Terraform configuration..."
    if terraform validate; then
        print_success "Terraform configuration is valid"
    else
        print_error "Terraform configuration validation failed"
        return 1
    fi
}

# Function to plan Terraform deployment
terraform_plan() {
    print_header "Planning Terraform Deployment"
    
    cd "$TERRAFORM_DIR"
    
    print_status "Creating Terraform execution plan..."
    if terraform plan -out=tfplan; then
        print_success "Terraform plan created successfully"
        print_info "Plan saved to: $TERRAFORM_DIR/tfplan"
    else
        print_error "Terraform planning failed"
        return 1
    fi
}

# Function to apply Terraform
terraform_apply() {
    print_header "Applying Terraform Configuration"
    
    cd "$TERRAFORM_DIR"
    
    if [[ -f "tfplan" ]]; then
        print_status "Applying Terraform plan..."
        if terraform apply tfplan; then
            print_success "Terraform applied successfully"
        else
            print_error "Terraform apply failed"
            return 1
        fi
    else
        print_status "Applying Terraform configuration..."
        if terraform apply -auto-approve; then
            print_success "Terraform applied successfully"
        else
            print_error "Terraform apply failed"
            return 1
        fi
    fi
    
    # Wait for VMs to be ready
    print_status "Waiting for VMs to be ready..."
    sleep 30
    
    # Test SSH connectivity to all VMs
    print_status "Testing SSH connectivity..."
    if [[ -f "$ANSIBLE_DIR/inventory.ini" ]]; then
        if ansible all -i "$ANSIBLE_DIR/inventory.ini" -m ping --timeout=10; then
            print_success "All VMs are SSH accessible"
        else
            print_warning "Some VMs may not be ready yet. Waiting longer..."
            sleep 60
            ansible all -i "$ANSIBLE_DIR/inventory.ini" -m ping --timeout=10
        fi
    fi
}

# Function to run Ansible configuration
run_ansible() {
    print_header "Running Ansible Configuration"
    
    cd "$ANSIBLE_DIR"
    
    # Check if inventory exists
    if [[ ! -f "inventory.ini" ]]; then
        print_error "Ansible inventory not found. Run Terraform first."
        return 1
    fi
    
    print_status "Running Ansible playbook..."
    if ansible-playbook -i inventory.ini site.yml --timeout=300; then
        print_success "Ansible configuration completed successfully"
    else
        print_error "Ansible configuration failed"
        return 1
    fi
}

# Function to test deployment
test_deployment() {
    print_header "Testing Deployment"
    
    cd "$ANSIBLE_DIR"
    
    if [[ ! -f "inventory.ini" ]]; then
        print_error "Ansible inventory not found. Deploy first."
        return 1
    fi
    
    # Test basic connectivity
    print_status "Testing VM connectivity..."
    if ansible all -i inventory.ini -m ping; then
        print_success "All VMs are accessible"
    else
        print_error "Some VMs are not accessible"
        return 1
    fi
    
    # Test SecBeat services
    print_status "Testing SecBeat services..."
    if ansible mitigation_nodes -i inventory.ini -m shell -a "systemctl is-active secbeat-mitigation || echo 'Not running'"; then
        print_info "Mitigation node services checked"
    fi
    
    if ansible orchestrator -i inventory.ini -m shell -a "systemctl is-active secbeat-orchestrator || echo 'Not running'"; then
        print_info "Orchestrator service checked"
    fi
    
    # Test monitoring
    print_status "Testing monitoring services..."
    if ansible monitoring -i inventory.ini -m shell -a "curl -f http://localhost:9090/-/healthy || echo 'Prometheus not ready'"; then
        print_info "Prometheus health checked"
    fi
    
    if ansible monitoring -i inventory.ini -m shell -a "curl -f http://localhost:3000/api/health || echo 'Grafana not ready'"; then
        print_info "Grafana health checked"
    fi
    
    print_success "Deployment tests completed"
}

# Function to show deployment status
show_status() {
    print_header "Deployment Status"
    
    cd "$TERRAFORM_DIR"
    
    # Show Terraform state
    if [[ -f "terraform.tfstate" ]]; then
        print_status "Terraform resources:"
        terraform show -json | jq -r '.values.root_module.resources[] | select(.type | startswith("proxmox_vm_qemu")) | .values | "\(.name) - \(.default_ipv4_address)"'
    else
        print_warning "No Terraform state found"
    fi
    
    # Show Ansible connectivity
    if [[ -f "$ANSIBLE_DIR/inventory.ini" ]]; then
        print_status "Testing Ansible connectivity..."
        cd "$ANSIBLE_DIR"
        ansible all -i inventory.ini -m ping | grep -E "(SUCCESS|UNREACHABLE|FAILED)"
    else
        print_warning "No Ansible inventory found"
    fi
}

# Function to destroy infrastructure
destroy_infrastructure() {
    print_header "Destroying Infrastructure"
    
    print_warning "This will destroy ALL SecBeat infrastructure!"
    if [[ "$FORCE" != "true" ]]; then
        read -p "Are you sure? (yes/no): " confirm
        if [[ "$confirm" != "yes" ]]; then
            print_info "Destruction cancelled"
            return 0
        fi
    fi
    
    cd "$TERRAFORM_DIR"
    
    if terraform destroy -auto-approve; then
        print_success "Infrastructure destroyed successfully"
    else
        print_error "Infrastructure destruction failed"
        return 1
    fi
}

# Main function
main() {
    local command=""
    local verbose=false
    local force=false
    local skip_terraform=false
    local skip_ansible=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -v|--verbose)
                verbose=true
                shift
                ;;
            -f|--force)
                force=true
                FORCE=true
                shift
                ;;
            --skip-terraform)
                skip_terraform=true
                shift
                ;;
            --skip-ansible)
                skip_ansible=true
                shift
                ;;
            init|plan|apply|configure|deploy|test|destroy|status|help)
                command="$1"
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
    
    # Set verbose mode
    if [[ "$verbose" == "true" ]]; then
        set -x
    fi
    
    # If no command specified, show usage
    if [[ -z "$command" ]]; then
        show_usage
        exit 1
    fi
    
    # Show header
    print_header "SecBeat Terraform + Ansible Deployment"
    print_info "Command: $command"
    print_info "Timestamp: $(date)"
    print_info "Log file: $DEPLOY_LOG"
    
    # Check prerequisites for most commands
    if [[ "$command" != "help" ]]; then
        check_prerequisites
    fi
    
    # Execute command
    case $command in
        init)
            terraform_init
            ;;
        plan)
            terraform_init
            terraform_plan
            ;;
        apply)
            if [[ "$skip_terraform" != "true" ]]; then
                terraform_init
                terraform_apply
            else
                print_info "Skipping Terraform (--skip-terraform)"
            fi
            ;;
        configure)
            if [[ "$skip_ansible" != "true" ]]; then
                run_ansible
            else
                print_info "Skipping Ansible (--skip-ansible)"
            fi
            ;;
        deploy)
            if [[ "$skip_terraform" != "true" ]]; then
                terraform_init
                terraform_apply
            else
                print_info "Skipping Terraform (--skip-terraform)"
            fi
            
            if [[ "$skip_ansible" != "true" ]]; then
                run_ansible
            else
                print_info "Skipping Ansible (--skip-ansible)"
            fi
            ;;
        test)
            test_deployment
            ;;
        destroy)
            destroy_infrastructure
            ;;
        status)
            show_status
            ;;
        help)
            show_usage
            ;;
        *)
            print_error "Invalid command: $command"
            show_usage
            exit 1
            ;;
    esac
    
    print_success "Command '$command' completed successfully"
}

# Script execution
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
