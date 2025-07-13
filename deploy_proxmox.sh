#!/bin/bash

# SecBeat Proxmox Deployment Orchestrator
# Main deployment script for automated Proxmox deployment

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOYMENT_DIR="$SCRIPT_DIR/deployment"
SCRIPTS_DIR="$DEPLOYMENT_DIR/scripts"
LOG_DIR="$SCRIPT_DIR/logs/deployment"

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
MAIN_LOG="$LOG_DIR/orchestrator_$(date +%Y%m%d_%H%M%S).log"

# Logging functions
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$MAIN_LOG"
}

print_header() {
    echo -e "${PURPLE}========================================${NC}"
    echo -e "${PURPLE}$1${NC}"
    echo -e "${PURPLE}========================================${NC}"
    log "HEADER: $1"
}

print_status() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
    log "STATUS: $1"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
    log "SUCCESS: $1"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    log "WARNING: $1"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
    log "ERROR: $1"
}

print_info() {
    echo -e "${CYAN}ℹ️  $1${NC}"
    log "INFO: $1"
}

# Function to show usage
show_usage() {
    cat << EOF
SecBeat Proxmox Deployment Orchestrator

Usage: $0 [OPTIONS] COMMAND

Commands:
    test        Run pre-deployment tests
    deploy      Run full deployment
    status      Check deployment status
    cleanup     Clean up deployment
    help        Show this help message

Options:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -c, --config FILE   Use custom configuration file
    --dry-run          Show what would be done without executing

Examples:
    $0 test                 # Run pre-deployment tests
    $0 deploy              # Deploy SecBeat to Proxmox
    $0 status              # Check status of deployed services
    $0 cleanup             # Clean up all deployed resources

Configuration:
    Default config: $DEPLOYMENT_DIR/proxmox-config.yml
    Logs directory: $LOG_DIR
    
For detailed information, see: README.md
EOF
}

# Function to validate environment
validate_environment() {
    print_header "Validating Environment"
    
    # Check if we're in the SecBeat directory
    if [ ! -f "$SCRIPT_DIR/Cargo.toml" ] || [ ! -d "$SCRIPT_DIR/mitigation-node" ]; then
        print_error "This script must be run from the SecBeat project root directory"
        exit 1
    fi
    
    # Check required scripts exist
    if [ ! -f "$SCRIPTS_DIR/test-proxmox.sh" ]; then
        print_error "Test script not found: $SCRIPTS_DIR/test-proxmox.sh"
        exit 1
    fi
    
    if [ ! -f "$SCRIPTS_DIR/deploy-proxmox.sh" ]; then
        print_error "Deployment script not found: $SCRIPTS_DIR/deploy-proxmox.sh"
        exit 1
    fi
    
    # Check configuration files
    if [ ! -f "$DEPLOYMENT_DIR/proxmox-config.yml" ]; then
        print_error "Configuration file not found: $DEPLOYMENT_DIR/proxmox-config.yml"
        exit 1
    fi
    
    # Check production configuration
    if [ ! -f "$SCRIPT_DIR/config/production.toml" ]; then
        print_error "Production configuration not found: $SCRIPT_DIR/config/production.toml"
        exit 1
    fi
    
    print_success "Environment validation passed"
}

# Function to run pre-deployment tests
run_tests() {
    print_header "Running Pre-Deployment Tests"
    
    print_status "Executing pre-deployment test script..."
    if "$SCRIPTS_DIR/test-proxmox.sh"; then
        print_success "Pre-deployment tests completed successfully"
        return 0
    else
        print_error "Pre-deployment tests failed"
        return 1
    fi
}

# Function to run full deployment
run_deployment() {
    print_header "Running Full Deployment"
    
    print_status "Starting deployment process..."
    print_info "This process may take 30-60 minutes depending on your Proxmox host performance"
    
    if "$SCRIPTS_DIR/deploy-proxmox.sh"; then
        print_success "Deployment completed successfully"
        show_deployment_info
        return 0
    else
        print_error "Deployment failed"
        print_info "Check deployment logs in: $LOG_DIR"
        return 1
    fi
}

# Function to show deployment information
show_deployment_info() {
    print_header "Deployment Information"
    
    echo "🌐 Access URLs:"
    echo "   Grafana Monitoring: http://192.168.100.209:3000"
    echo "   Prometheus Metrics: http://192.168.100.209:9090" 
    echo "   Load Balancer Stats: http://192.168.100.207:8404/stats"
    echo ""
    echo "🔐 Default Credentials:"
    echo "   Grafana: admin / secbeat123"
    echo ""
    echo "🖥️  VM Access:"
    echo "   SSH: ssh -i ~/.ssh/id_rsa secbeat@<vm_ip>"
    echo "   All VMs use 'secbeat' user with sudo privileges"
    echo ""
    echo "📊 IP Addresses:"
    echo "   Mitigation Nodes: 192.168.100.200-202 (1GB RAM, 8GB disk each)"
    echo "   Orchestrator: 192.168.100.203 (1GB RAM, 8GB disk)"
    echo "   NATS Cluster: 192.168.100.204-206 (512MB RAM, 6GB disk each)"
    echo "   Load Balancers: 192.168.100.207-208 (512MB RAM, 6GB disk each)"
    echo "   Monitoring: 192.168.100.209 (1.5GB RAM, 12GB disk)"
    echo ""
    echo "💾 Total Resources Used:"
    echo "   RAM: 7.5GB / 8GB available"
    echo "   Disk: 80GB / 100GB available"
    echo ""
    echo "📝 Logs and Reports:"
    echo "   Deployment logs: $LOG_DIR"
    echo "   Latest report: $(ls -t $LOG_DIR/deployment_report_*.md 2>/dev/null | head -1 || echo 'Not found')"
}

# Function to check deployment status
check_status() {
    print_header "Checking Deployment Status"
    
    # Check if SSH key exists
    if [ ! -f "$HOME/.ssh/id_rsa" ]; then
        print_error "SSH key not found at ~/.ssh/id_rsa"
        return 1
    fi
    
    # Define VMs to check
    declare -A VMS
    VMS["mitigation-1"]="192.168.100.200"
    VMS["mitigation-2"]="192.168.100.201"
    VMS["mitigation-3"]="192.168.100.202"
    VMS["orchestrator"]="192.168.100.203"
    VMS["nats-1"]="192.168.100.204"
    VMS["nats-2"]="192.168.100.205"
    VMS["nats-3"]="192.168.100.206"
    VMS["loadbalancer-1"]="192.168.100.207"
    VMS["loadbalancer-2"]="192.168.100.208"
    VMS["monitoring"]="192.168.100.209"
    
    print_status "Checking VM connectivity..."
    local all_online=true
    
    for vm_name in "${!VMS[@]}"; do
        local ip="${VMS[$vm_name]}"
        if ssh -i ~/.ssh/id_rsa -o ConnectTimeout=5 -o StrictHostKeyChecking=no "secbeat@$ip" "echo 'online'" >/dev/null 2>&1; then
            print_success "$vm_name ($ip) - Online"
        else
            print_error "$vm_name ($ip) - Offline"
            all_online=false
        fi
    done
    
    if $all_online; then
        print_status "Checking service health..."
        
        # Check mitigation services
        for i in {200..202}; do
            local ip="192.168.100.$i"
            if ssh -i ~/.ssh/id_rsa "secbeat@$ip" "sudo systemctl is-active --quiet secbeat-mitigation" 2>/dev/null; then
                print_success "SecBeat mitigation service running on $ip"
            else
                print_warning "SecBeat mitigation service not running on $ip"
            fi
        done
        
        # Check monitoring services
        if ssh -i ~/.ssh/id_rsa "secbeat@192.168.100.209" "curl -f http://localhost:9090/-/healthy" >/dev/null 2>&1; then
            print_success "Prometheus monitoring healthy"
        else
            print_warning "Prometheus monitoring not responding"
        fi
        
        if ssh -i ~/.ssh/id_rsa "secbeat@192.168.100.209" "curl -f http://localhost:3000/api/health" >/dev/null 2>&1; then
            print_success "Grafana dashboard healthy"
        else
            print_warning "Grafana dashboard not responding"
        fi
        
        print_success "Status check completed"
    else
        print_error "Some VMs are offline - deployment may not be complete"
        return 1
    fi
}

# Function to cleanup deployment
cleanup_deployment() {
    print_header "Cleaning Up Deployment"
    
    print_warning "This will destroy ALL SecBeat VMs and data!"
    read -p "Are you sure you want to continue? [y/N]: " -n 1 -r
    echo
    
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Cleanup cancelled"
        return 0
    fi
    
    print_status "Stopping and destroying VMs..."
    
    # VM IDs to cleanup
    local vm_ids=(101 102 103 110 121 122 123 131 132 140)
    
    for vm_id in "${vm_ids[@]}"; do
        if ssh -i ~/.ssh/id_rsa "root@192.168.100.23" "qm status $vm_id" >/dev/null 2>&1; then
            print_status "Destroying VM $vm_id"
            ssh -i ~/.ssh/id_rsa "root@192.168.100.23" "qm stop $vm_id || true"
            ssh -i ~/.ssh/id_rsa "root@192.168.100.23" "qm destroy $vm_id || true"
        fi
    done
    
    print_success "Cleanup completed"
}

# Main function
main() {
    local command=""
    local verbose=false
    local dry_run=false
    local config_file=""
    
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
            -c|--config)
                config_file="$2"
                shift 2
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            test|deploy|status|cleanup|help)
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
    
    # If no command specified, show usage
    if [ -z "$command" ]; then
        show_usage
        exit 1
    fi
    
    # Show header
    print_header "SecBeat Proxmox Deployment Orchestrator"
    print_info "Command: $command"
    print_info "Timestamp: $(date)"
    print_info "Log file: $MAIN_LOG"
    
    # Validate environment for all commands except help
    if [ "$command" != "help" ]; then
        validate_environment
    fi
    
    # Execute command
    case $command in
        test)
            run_tests
            ;;
        deploy)
            print_info "Starting full deployment process..."
            print_info "Phase 1: Pre-deployment tests"
            if run_tests; then
                print_info "Phase 2: Full deployment"
                run_deployment
            else
                print_error "Pre-deployment tests failed - aborting deployment"
                exit 1
            fi
            ;;
        status)
            check_status
            ;;
        cleanup)
            cleanup_deployment
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
}

# Script execution
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi