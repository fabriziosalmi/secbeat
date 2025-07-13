#!/bin/bash

# SecBeat Proxmox Deployment Script
# Automated VM creation, software installation, and service deployment

set -e

# Configuration
PROXMOX_HOST="192.168.100.23"
PROXMOX_USER="root@pam"
SSH_KEY_PATH="$HOME/.ssh/id_rsa"
ISO_NAME="ubuntu-24.04.2-live-server-amd64.iso"
ISO_PATH="/var/lib/vz/template/iso/ubuntu-24.04.2-live-server-amd64.iso"
STORAGE="local"
BRIDGE="vmbr0"

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEPLOYMENT_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$DEPLOYMENT_DIR")"
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
DEPLOY_LOG="$LOG_DIR/deployment_$(date +%Y%m%d_%H%M%S).log"

# Logging functions
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$DEPLOY_LOG"
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

# VM Configuration - Optimized for 8GB RAM / 100GB Disk - Single Network
declare -A VM_CONFIGS
VM_CONFIGS[mitigation-1]="101:mitigation-node:1:1024:8:192.168.100.200"    # 1GB RAM, 8GB disk
VM_CONFIGS[mitigation-2]="102:mitigation-node:1:1024:8:192.168.100.201"    # 1GB RAM, 8GB disk
VM_CONFIGS[mitigation-3]="103:mitigation-node:1:1024:8:192.168.100.202"    # 1GB RAM, 8GB disk
VM_CONFIGS[orchestrator]="110:orchestrator-node:1:1024:8:192.168.100.203"  # 1GB RAM, 8GB disk
VM_CONFIGS[nats-1]="121:nats-cluster:1:512:6:192.168.100.204"              # 512MB RAM, 6GB disk
VM_CONFIGS[nats-2]="122:nats-cluster:1:512:6:192.168.100.205"              # 512MB RAM, 6GB disk
VM_CONFIGS[nats-3]="123:nats-cluster:1:512:6:192.168.100.206"              # 512MB RAM, 6GB disk
VM_CONFIGS[loadbalancer-1]="131:load-balancer:1:512:6:192.168.100.207"     # 512MB RAM, 6GB disk
VM_CONFIGS[loadbalancer-2]="132:load-balancer:1:512:6:192.168.100.208"     # 512MB RAM, 6GB disk
VM_CONFIGS[monitoring]="140:monitoring:1:1536:12:192.168.100.209"          # 1.5GB RAM, 12GB disk

# Total resources: 7.5GB RAM, 80GB disk

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

# Function to check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"
    
    # Check SSH key
    if [ ! -f "$SSH_KEY_PATH" ]; then
        print_error "SSH key not found at $SSH_KEY_PATH"
        print_info "Generate SSH key with: ssh-keygen -t rsa -b 4096 -f $SSH_KEY_PATH"
        exit 1
    fi
    
    # Check SSH connectivity
    if ! ssh -i "$SSH_KEY_PATH" -o StrictHostKeyChecking=no -o ConnectTimeout=10 "$PROXMOX_USER@$PROXMOX_HOST" "echo 'SSH connection successful'" >/dev/null 2>&1; then
        print_error "Cannot connect to Proxmox host $PROXMOX_HOST"
        print_info "Ensure SSH key is added to Proxmox host authorized_keys"
        exit 1
    fi
    
    # Check if ISO exists
    if ! proxmox_exec "test -f $ISO_PATH" "Checking ISO availability"; then
        print_error "ISO $ISO_NAME not found at $ISO_PATH"
        print_info "Expected path: $ISO_PATH"
        print_info "Upload ISO to Proxmox: datacenter -> storage -> local -> ISO Images"
        exit 1
    fi
    
    print_success "All prerequisites met"
}

# Function to create cloud-init user data
create_cloud_init_config() {
    local vm_name="$1"
    local ip_address="$2"
    local vm_type="$3"
    
    local config_file="/tmp/user-data-$vm_name.yml"
    
    cat > "$config_file" << EOF
#cloud-config
hostname: $vm_name
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
  - ufw
  - fail2ban
  - unzip
  - jq

network:
  version: 2
  ethernets:
    ens18:
      dhcp4: false
      addresses: [$ip_address/24]
      gateway4: 192.168.100.1
      nameservers:
        addresses: [8.8.8.8, 8.8.4.4]

write_files:
  - path: /etc/ssh/sshd_config.d/99-secbeat.conf
    content: |
      PasswordAuthentication no
      PubkeyAuthentication yes
      PermitRootLogin no
    permissions: '0644'

  - path: /opt/secbeat/vm-type
    content: |
      $vm_type
    permissions: '0644'

runcmd:
  - systemctl restart sshd
  - ufw --force enable
  - ufw allow ssh
  - mkdir -p /opt/secbeat
  - chown secbeat:secbeat /opt/secbeat
  - systemctl enable fail2ban
  - systemctl start fail2ban

final_message: "SecBeat VM $vm_name is ready for deployment"
EOF

    echo "$config_file"
}

# Function to create a VM
create_vm() {
    local vm_name="$1"
    local vm_config="$2"
    
    IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
    
    print_status "Creating VM: $vm_name (ID: $vm_id, Type: $vm_type)"
    
    # Create cloud-init config
    local cloud_init_config
    cloud_init_config=$(create_cloud_init_config "$vm_name" "$ip" "$vm_type")
    
    # Copy cloud-init config to Proxmox
    scp -i "$SSH_KEY_PATH" "$cloud_init_config" "$PROXMOX_USER@$PROXMOX_HOST:/tmp/"
    
    # Create VM
    proxmox_exec "qm create $vm_id --name $vm_name --memory $memory --cores $cores --net0 virtio,bridge=$BRIDGE --scsihw virtio-scsi-pci --scsi0 $STORAGE:${disk}" "Creating VM $vm_name"
    
    # Import disk
    proxmox_exec "qm importdisk $vm_id $ISO_PATH $STORAGE" "Importing disk for VM $vm_name"
    
    # Configure VM
    proxmox_exec "qm set $vm_id --scsi0 $STORAGE:vm-$vm_id-disk-0 --ide2 $STORAGE:cloudinit --boot c --bootdisk scsi0 --serial0 socket --vga serial0" "Configuring VM $vm_name"
    
    # Set cloud-init config
    proxmox_exec "qm set $vm_id --cicustom user=local:snippets/user-data-$vm_name.yml" "Setting cloud-init config for VM $vm_name"
    
    # Copy cloud-init to snippets directory
    proxmox_exec "cp /tmp/user-data-$vm_name.yml /var/lib/vz/snippets/" "Copying cloud-init config to snippets"
    
    # Clean up temporary file
    rm -f "$cloud_init_config"
    
    print_success "VM $vm_name created successfully"
}

# Function to start VM and wait for it to be ready
start_vm_and_wait() {
    local vm_name="$1"
    local vm_config="$2"
    
    IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
    
    print_status "Starting VM: $vm_name"
    proxmox_exec "qm start $vm_id" "Starting VM $vm_name"
    
    print_status "Waiting for VM $vm_name to be ready..."
    local max_attempts=60
    local attempt=1
    
    while [ $attempt -le $max_attempts ]; do
        if ssh -i "$SSH_KEY_PATH" -o StrictHostKeyChecking=no -o ConnectTimeout=5 "secbeat@$ip" "echo 'VM ready'" >/dev/null 2>&1; then
            print_success "VM $vm_name is ready"
            return 0
        fi
        
        print_info "Attempt $attempt/$max_attempts - VM $vm_name not ready yet..."
        sleep 10
        ((attempt++))
    done
    
    print_error "VM $vm_name failed to become ready after $max_attempts attempts"
    return 1
}

# Function to install software on VM
install_software() {
    local vm_name="$1"
    local vm_config="$2"
    
    IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
    
    print_status "Installing software on $vm_name ($vm_type)"
    
    # Create installation script based on VM type
    local install_script="/tmp/install-$vm_name.sh"
    
    case "$vm_type" in
        "mitigation-node"|"orchestrator-node")
            create_rust_install_script "$install_script"
            ;;
        "nats-cluster")
            create_nats_install_script "$install_script"
            ;;
        "load-balancer")
            create_nginx_install_script "$install_script"
            ;;
        "monitoring")
            create_monitoring_install_script "$install_script"
            ;;
        *)
            print_warning "Unknown VM type: $vm_type, skipping software installation"
            return 0
            ;;
    esac
    
    # Copy and execute installation script
    scp -i "$SSH_KEY_PATH" "$install_script" "secbeat@$ip:/tmp/install.sh"
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "chmod +x /tmp/install.sh && sudo /tmp/install.sh"
    
    rm -f "$install_script"
    print_success "Software installation completed on $vm_name"
}

# Function to create Rust installation script
create_rust_install_script() {
    local script_file="$1"
    
    cat > "$script_file" << 'EOF'
#!/bin/bash
set -e

# Install Rust and dependencies - Lightweight version
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
source ~/.cargo/env

# Install essential system dependencies only
apt-get update
apt-get install -y build-essential pkg-config libssl-dev

# Skip Docker installation for mitigation/orchestrator nodes to save resources
# Docker will only be installed on monitoring node

# Create SecBeat directories
mkdir -p /opt/secbeat/{config,logs,data}
chown -R secbeat:secbeat /opt/secbeat

echo "Rust and dependencies installed successfully"
EOF
}

# Function to create NATS installation script
create_nats_install_script() {
    local script_file="$1"
    
    cat > "$script_file" << 'EOF'
#!/bin/bash
set -e

# Install NATS Server
cd /tmp
wget https://github.com/nats-io/nats-server/releases/download/v2.10.0/nats-server-v2.10.0-linux-amd64.tar.gz
tar -xzf nats-server-v2.10.0-linux-amd64.tar.gz
cp nats-server-v2.10.0-linux-amd64/nats-server /usr/local/bin/
chmod +x /usr/local/bin/nats-server

# Create NATS user and directories
useradd --system --shell /bin/false --home /var/lib/nats --create-home nats
mkdir -p /etc/nats /var/log/nats
chown nats:nats /var/lib/nats /var/log/nats

# Create NATS systemd service
cat > /etc/systemd/system/nats.service << 'EOFSERVICE'
[Unit]
Description=NATS Server
After=network.target

[Service]
Type=simple
User=nats
Group=nats
ExecStart=/usr/local/bin/nats-server -c /etc/nats/nats.conf
ExecReload=/bin/kill -s HUP $MAINPID
Restart=always
RestartSec=5s

[Install]
WantedBy=multi-user.target
EOFSERVICE

# Enable NATS service
systemctl daemon-reload
systemctl enable nats

echo "NATS Server installed successfully"
EOF
}

# Function to create Nginx installation script
create_nginx_install_script() {
    local script_file="$1"
    
    cat > "$script_file" << 'EOF'
#!/bin/bash
set -e

# Install Nginx
apt-get update
apt-get install -y nginx

# Install HAProxy for advanced load balancing
apt-get install -y haproxy

# Create configuration directories
mkdir -p /etc/nginx/sites-available /etc/nginx/sites-enabled
mkdir -p /var/log/nginx /var/log/haproxy

# Enable services
systemctl enable nginx
systemctl enable haproxy

echo "Load balancer software installed successfully"
EOF
}

# Function to create monitoring installation script
create_monitoring_install_script() {
    local script_file="$1"
    
    cat > "$script_file" << 'EOF'
#!/bin/bash
set -e

# Install Docker and Docker Compose
curl -fsSL https://get.docker.com | sh
usermod -aG docker secbeat

curl -L "https://github.com/docker/compose/releases/download/v2.20.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Create monitoring directories
mkdir -p /opt/monitoring/{prometheus,grafana,alertmanager}
chown -R secbeat:secbeat /opt/monitoring

echo "Monitoring stack prerequisites installed successfully"
EOF
}

# Function to deploy SecBeat code
deploy_secbeat_code() {
    local vm_name="$1"
    local vm_config="$2"
    
    IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
    
    if [[ "$vm_type" =~ ^(mitigation-node|orchestrator-node)$ ]]; then
        print_status "Deploying SecBeat code to $vm_name"
        
        # Create deployment archive
        local archive_file="/tmp/secbeat-deployment.tar.gz"
        cd "$PROJECT_ROOT"
        tar --exclude='target' --exclude='.git' --exclude='logs' -czf "$archive_file" .
        
        # Copy and extract code
        scp -i "$SSH_KEY_PATH" "$archive_file" "secbeat@$ip:/tmp/"
        ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "cd /opt/secbeat && tar -xzf /tmp/secbeat-deployment.tar.gz --strip-components=1"
        
        # Build the code
        ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "cd /opt/secbeat && source ~/.cargo/env && cargo build --release"
        
        rm -f "$archive_file"
        print_success "SecBeat code deployed to $vm_name"
    fi
}

# Function to configure services
configure_services() {
    print_header "Configuring Services"
    
    for vm_name in "${!VM_CONFIGS[@]}"; do
        local vm_config="${VM_CONFIGS[$vm_name]}"
        IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
        
        case "$vm_type" in
            "mitigation-node")
                configure_mitigation_node "$vm_name" "$ip"
                ;;
            "orchestrator-node")
                configure_orchestrator_node "$vm_name" "$ip"
                ;;
            "nats-cluster")
                configure_nats_cluster "$vm_name" "$ip"
                ;;
            "load-balancer")
                configure_load_balancer "$vm_name" "$ip"
                ;;
            "monitoring")
                configure_monitoring "$vm_name" "$ip"
                ;;
        esac
    done
}

# Function to configure mitigation node
configure_mitigation_node() {
    local vm_name="$1"
    local ip="$2"
    
    print_status "Configuring mitigation node: $vm_name"
    
    # Copy production config
    scp -i "$SSH_KEY_PATH" "$PROJECT_ROOT/config/production.toml" "secbeat@$ip:/opt/secbeat/config/"
    
    # Create systemd service
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo tee /etc/systemd/system/secbeat-mitigation.service" << 'EOF'
[Unit]
Description=SecBeat Mitigation Node
After=network.target

[Service]
Type=simple
User=secbeat
WorkingDirectory=/opt/secbeat
ExecStart=/opt/secbeat/target/release/mitigation-node --config /opt/secbeat/config/production.toml
Restart=always
RestartSec=10s
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
    
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo systemctl daemon-reload && sudo systemctl enable secbeat-mitigation"
}

# Function to configure orchestrator node
configure_orchestrator_node() {
    local vm_name="$1"
    local ip="$2"
    
    print_status "Configuring orchestrator node: $vm_name"
    
    # Create systemd service
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo tee /etc/systemd/system/secbeat-orchestrator.service" << 'EOF'
[Unit]
Description=SecBeat Orchestrator Node
After=network.target

[Service]
Type=simple
User=secbeat
WorkingDirectory=/opt/secbeat
ExecStart=/opt/secbeat/target/release/orchestrator-node
Restart=always
RestartSec=10s
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
    
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo systemctl daemon-reload && sudo systemctl enable secbeat-orchestrator"
}

# Function to configure NATS cluster
configure_nats_cluster() {
    local vm_name="$1"
    local ip="$2"
    
    print_status "Configuring NATS cluster node: $vm_name"
    
    # Create NATS configuration
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo tee /etc/nats/nats.conf" << EOF
# NATS Server Configuration
port: 4222
http_port: 8222

# Cluster configuration
cluster {
  name: secbeat-cluster
  listen: 0.0.0.0:6222
  routes: [
    nats://192.168.100.204:6222
    nats://192.168.100.205:6222
    nats://192.168.100.206:6222
  ]
}

# Authentication
authorization {
  token: "\$NATS_AUTH_TOKEN"
}

# Logging
log_file: "/var/log/nats/nats.log"
logtime: true
debug: false
trace: false
EOF
    
    # Start NATS service
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo systemctl start nats"
}

# Function to configure load balancer
configure_load_balancer() {
    local vm_name="$1"
    local ip="$2"
    
    print_status "Configuring load balancer: $vm_name"
    
    # Create HAProxy configuration
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo tee /etc/haproxy/haproxy.cfg" << 'EOF'
global
    daemon
    log stdout local0

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms
    log global
    option httplog

frontend secbeat_frontend
    bind *:443 ssl crt /etc/ssl/certs/secbeat.pem
    redirect scheme https if !{ ssl_fc }
    default_backend secbeat_mitigation_nodes

backend secbeat_mitigation_nodes
    balance roundrobin
    option httpchk GET /health
    server mitigation-1 192.168.100.200:8443 check
    server mitigation-2 192.168.100.201:8443 check
    server mitigation-3 192.168.100.202:8443 check

frontend stats
    bind *:8404
    stats enable
    stats uri /stats
    stats refresh 30s
EOF
    
    # Start HAProxy service
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo systemctl start haproxy"
}

# Function to configure monitoring
configure_monitoring() {
    local vm_name="$1"
    local ip="$2"
    
    print_status "Configuring monitoring stack: $vm_name"
    
    # Create Docker Compose file for monitoring stack - Lightweight version
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "tee /opt/monitoring/docker-compose.yml" << 'EOF'
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus:/etc/prometheus
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=7d'  # Reduced retention for low disk space
      - '--storage.tsdb.retention.size=2GB'  # Limit storage size
      - '--web.enable-lifecycle'
    restart: unless-stopped
    mem_limit: 512m  # Limit memory usage

  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    ports:
      - "3000:3000"
    volumes:
      - grafana_data:/var/lib/grafana
      - ./grafana:/etc/grafana/provisioning
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=secbeat123
      - GF_INSTALL_PLUGINS=
    restart: unless-stopped
    mem_limit: 256m  # Limit memory usage

  # Removed AlertManager to save resources - can be added later if needed
  
volumes:
  prometheus_data:
  grafana_data:
EOF
    
    # Start monitoring stack
    ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "cd /opt/monitoring && docker-compose up -d"
}

# Function to run deployment tests
run_deployment_tests() {
    print_header "Running Deployment Tests"
    
    # Test mitigation nodes
    for vm_name in mitigation-1 mitigation-2 mitigation-3; do
        local vm_config="${VM_CONFIGS[$vm_name]}"
        IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
        
        print_status "Testing mitigation node: $vm_name"
        
        # Test SSH connectivity
        if ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "echo 'SSH test successful'"; then
            print_success "SSH connectivity test passed for $vm_name"
        else
            print_error "SSH connectivity test failed for $vm_name"
        fi
        
        # Test service status
        if ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo systemctl is-active --quiet secbeat-mitigation"; then
            print_success "SecBeat mitigation service is running on $vm_name"
        else
            print_warning "SecBeat mitigation service is not running on $vm_name"
        fi
        
        # Test health endpoint
        if ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "curl -f http://localhost:9192/health" >/dev/null 2>&1; then
            print_success "Health endpoint test passed for $vm_name"
        else
            print_warning "Health endpoint test failed for $vm_name"
        fi
    done
    
    # Test load balancers
    for vm_name in loadbalancer-1 loadbalancer-2; do
        local vm_config="${VM_CONFIGS[$vm_name]}"
        IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
        
        print_status "Testing load balancer: $vm_name"
        
        if ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "curl -f http://localhost:8404/stats" >/dev/null 2>&1; then
            print_success "HAProxy stats endpoint test passed for $vm_name"
        else
            print_warning "HAProxy stats endpoint test failed for $vm_name"
        fi
    done
    
    # Test monitoring
    local monitoring_ip="192.168.300.10"
    print_status "Testing monitoring stack"
    
    if ssh -i "$SSH_KEY_PATH" "secbeat@$monitoring_ip" "curl -f http://localhost:9090/-/healthy" >/dev/null 2>&1; then
        print_success "Prometheus health check passed"
    else
        print_warning "Prometheus health check failed"
    fi
    
    if ssh -i "$SSH_KEY_PATH" "secbeat@$monitoring_ip" "curl -f http://localhost:3000/api/health" >/dev/null 2>&1; then
        print_success "Grafana health check passed"
    else
        print_warning "Grafana health check failed"
    fi
}

# Function to generate deployment report
generate_deployment_report() {
    print_header "Generating Deployment Report"
    
    local report_file="$LOG_DIR/deployment_report_$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# SecBeat Proxmox Deployment Report

**Deployment Date:** $(date)
**Proxmox Host:** $PROXMOX_HOST
**Deployment Log:** $DEPLOY_LOG

## VM Deployment Summary

| VM Name | VM ID | Type | IP Address | Status |
|---------|-------|------|------------|--------|
EOF

    for vm_name in "${!VM_CONFIGS[@]}"; do
        local vm_config="${VM_CONFIGS[$vm_name]}"
        IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
        
        # Check VM status
        local status="Unknown"
        if ssh -i "$SSH_KEY_PATH" -o ConnectTimeout=5 "secbeat@$ip" "echo 'online'" >/dev/null 2>&1; then
            status="Online"
        else
            status="Offline"
        fi
        
        echo "| $vm_name | $vm_id | $vm_type | $ip | $status |" >> "$report_file"
    done
    
    cat >> "$report_file" << EOF

## Service Status

### Mitigation Nodes
EOF

    for vm_name in mitigation-1 mitigation-2 mitigation-3; do
        local vm_config="${VM_CONFIGS[$vm_name]}"
        IFS=':' read -r vm_id vm_type cores memory disk ip <<< "$vm_config"
        
        echo "- **$vm_name ($ip):**" >> "$report_file"
        
        if ssh -i "$SSH_KEY_PATH" "secbeat@$ip" "sudo systemctl is-active --quiet secbeat-mitigation" 2>/dev/null; then
            echo "  - SecBeat Service: ✅ Running" >> "$report_file"
        else
            echo "  - SecBeat Service: ❌ Not Running" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF

### Monitoring Stack
- **Prometheus:** http://192.168.300.10:9090
- **Grafana:** http://192.168.300.10:3000 (admin/secbeat123)
- **AlertManager:** http://192.168.300.10:9093

### Load Balancers
- **HAProxy Stats:** http://192.168.200.40:8404/stats
- **HAProxy Stats:** http://192.168.200.41:8404/stats

## Next Steps

1. Configure SSL certificates for load balancers
2. Set up monitoring dashboards in Grafana
3. Configure alerting rules in AlertManager
4. Run comprehensive load testing
5. Set up backup procedures

## Access Information

- SSH Access: \`ssh -i $SSH_KEY_PATH secbeat@<vm_ip>\`
- All VMs use user 'secbeat' with sudo privileges
- Service logs: \`journalctl -u service-name -f\`

EOF
    
    print_success "Deployment report generated: $report_file"
    print_info "View report: cat $report_file"
}

# Main deployment function
main() {
    print_header "SecBeat Proxmox Deployment"
    print_info "Target Host: $PROXMOX_HOST"
    print_info "Deployment Log: $DEPLOY_LOG"
    
    # Check prerequisites
    check_prerequisites
    
    # Create VMs
    print_header "Creating Virtual Machines"
    for vm_name in "${!VM_CONFIGS[@]}"; do
        create_vm "$vm_name" "${VM_CONFIGS[$vm_name]}"
    done
    
    # Start VMs and wait for them to be ready
    print_header "Starting Virtual Machines"
    for vm_name in "${!VM_CONFIGS[@]}"; do
        start_vm_and_wait "$vm_name" "${VM_CONFIGS[$vm_name]}"
    done
    
    # Install software
    print_header "Installing Software"
    for vm_name in "${!VM_CONFIGS[@]}"; do
        install_software "$vm_name" "${VM_CONFIGS[$vm_name]}"
    done
    
    # Deploy SecBeat code
    print_header "Deploying SecBeat Code"
    for vm_name in "${!VM_CONFIGS[@]}"; do
        deploy_secbeat_code "$vm_name" "${VM_CONFIGS[$vm_name]}"
    done
    
    # Configure services
    configure_services
    
    # Run tests
    run_deployment_tests
    
    # Generate report
    generate_deployment_report
    
    print_header "Deployment Complete"
    print_success "SecBeat has been successfully deployed to Proxmox!"
    print_info "Check the deployment report for detailed information"
    print_info "Access Grafana at: http://192.168.300.10:3000 (admin/secbeat123)"
    print_info "Access Prometheus at: http://192.168.300.10:9090"
}

# Script execution
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
