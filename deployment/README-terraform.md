# SecBeat Terraform + Ansible Deployment Guide

This directory contains a modern, robust deployment automation system for SecBeat using Terraform for infrastructure provisioning and Ansible for configuration management.

## 🏗️ Architecture

The deployment system provides:
- **Terraform** for VM provisioning and infrastructure management
- **Ansible** for software installation and configuration
- **Automated SSL certificate generation**
- **Complete monitoring stack deployment**
- **Production-ready security configuration**

## 📋 Prerequisites

### Required Software
- **Terraform** >= 1.0 ([Download](https://terraform.io/downloads))
- **Ansible** >= 2.9 ([Install](https://docs.ansible.com/ansible/latest/installation_guide/intro_installation.html))
- **jq** for JSON processing
- **SSH key pair** at `~/.ssh/id_rsa`

### Proxmox Requirements
- **Proxmox VE 7.0+** with API access
- **Ubuntu 24.04 cloud image** at `/var/lib/vz/template/iso/ubuntu-24.04-server-cloudimg-amd64.img`
- **SSH access** to Proxmox host configured
- **Sufficient resources**: 8+ GB RAM, 100+ GB storage

### Quick Software Installation (macOS)
```bash
# Install Terraform
brew install terraform

# Install Ansible
brew install ansible

# Install jq
brew install jq

# Verify installations
terraform version
ansible --version
jq --version
```

### Quick Software Installation (Ubuntu/Debian)
```bash
# Install Terraform
curl -fsSL https://apt.releases.hashicorp.com/gpg | sudo apt-key add -
sudo apt-add-repository "deb [arch=amd64] https://apt.releases.hashicorp.com $(lsb_release -cs) main"
sudo apt-get update && sudo apt-get install terraform

# Install Ansible
sudo apt update
sudo apt install ansible

# Install jq
sudo apt install jq
```

## 🚀 Quick Start

### 1. Configure Terraform Variables

```bash
# Copy the example configuration
cp terraform/terraform.tfvars.example terraform/terraform.tfvars

# Edit with your Proxmox details
vim terraform/terraform.tfvars
```

Required variables in `terraform.tfvars`:
```hcl
proxmox_host     = "192.168.100.23"
proxmox_user     = "root"
proxmox_password = "your_proxmox_password"
ssh_public_key   = "ssh-rsa AAAAB3NzaC1yc2EAAAA..."  # Content of ~/.ssh/id_rsa.pub
```

### 2. Setup SSH Access

```bash
# Generate SSH key if needed
ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa

# Test SSH access to Proxmox
ssh root@192.168.100.23

# Get your public key for terraform.tfvars
cat ~/.ssh/id_rsa.pub
```

### 3. Deploy the Infrastructure

```bash
# Initialize Terraform and check prerequisites
./deploy.sh init

# Show what will be created
./deploy.sh plan

# Deploy everything (infrastructure + configuration)
./deploy.sh deploy

# Or deploy in stages:
./deploy.sh apply      # Create VMs with Terraform
./deploy.sh configure  # Configure with Ansible
```

## 🛠️ Deployment Commands

### Primary Commands

| Command | Description |
|---------|-------------|
| `./deploy.sh init` | Initialize Terraform and check prerequisites |
| `./deploy.sh plan` | Show Terraform execution plan |
| `./deploy.sh deploy` | Complete deployment (Terraform + Ansible) |
| `./deploy.sh test` | Run deployment validation tests |
| `./deploy.sh status` | Check current deployment status |
| `./deploy.sh destroy` | Destroy all infrastructure |

### Advanced Commands

| Command | Description |
|---------|-------------|
| `./deploy.sh apply` | Run Terraform only |
| `./deploy.sh configure` | Run Ansible only |
| `./deploy.sh apply --skip-ansible` | Terraform without Ansible |
| `./deploy.sh configure --skip-terraform` | Ansible without Terraform |

### Command Options

| Option | Description |
|--------|-------------|
| `-v, --verbose` | Enable verbose output |
| `-f, --force` | Skip confirmation prompts |
| `--skip-terraform` | Skip Terraform operations |
| `--skip-ansible` | Skip Ansible operations |

## 📊 Infrastructure Layout

### VM Configuration (Optimized for 8GB RAM / 100GB Disk)

| Component | Count | IP Range | RAM | Disk | Purpose |
|-----------|-------|----------|-----|------|---------|
| **Mitigation Nodes** | 3 | 192.168.100.200-202 | 1GB each | 8GB each | DDoS protection & WAF |
| **Orchestrator** | 1 | 192.168.100.203 | 1GB | 8GB | Central coordination |
| **NATS Cluster** | 3 | 192.168.100.204-206 | 512MB each | 6GB each | Event messaging |
| **Load Balancers** | 2 | 192.168.100.207-208 | 512MB each | 6GB each | Traffic distribution |
| **Monitoring** | 1 | 192.168.100.209 | 1.5GB | 12GB | Prometheus + Grafana |

**Total Resources**: 7.5GB RAM, 80GB disk

### Network Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Proxmox Host (192.168.100.23)           │
├─────────────────────────────────────────────────────────────┤
│  Single Network (192.168.100.0/24)                         │
│  VM IP Range: 192.168.100.200-220                          │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Configuration

### Terraform Configuration

The main Terraform configuration is in `terraform/main.tf`. Key features:

- **VM Templates**: Uses Ubuntu 24.04 cloud images
- **Resource Optimization**: Configured for smaller environments
- **Network Configuration**: Single subnet for simplicity
- **SSL Certificates**: Auto-generated TLS certificates
- **Ansible Integration**: Generates inventory automatically

### Ansible Configuration

The Ansible playbook `ansible/site.yml` handles:

- **Base System Setup**: Package installation, security configuration
- **SecBeat Deployment**: Source code deployment and compilation
- **Service Configuration**: Systemd services for all components
- **Monitoring Setup**: Prometheus and Grafana deployment
- **Network Security**: Firewall and fail2ban configuration

### Template Files

| Template | Purpose |
|----------|---------|
| `templates/mitigation-node.service.j2` | SecBeat mitigation service |
| `templates/orchestrator.service.j2` | SecBeat orchestrator service |
| `templates/nats.conf.j2` | NATS cluster configuration |
| `templates/haproxy.cfg.j2` | Load balancer configuration |
| `templates/prometheus.yml.j2` | Monitoring configuration |

## 📈 Monitoring & Access

### Service Access

After deployment, access services at:

- **Grafana**: http://192.168.100.209:3000 (admin/secbeat123)
- **Prometheus**: http://192.168.100.209:9090
- **HAProxy Stats**: http://192.168.100.207:8404/stats
- **SecBeat API**: https://192.168.100.207/api

### SSH Access

```bash
# Access any VM
ssh -i ~/.ssh/id_rsa secbeat@192.168.100.XXX

# Check service status
sudo systemctl status secbeat-mitigation
sudo systemctl status secbeat-orchestrator
sudo systemctl status nats
```

### Log Files

| Service | Log Location |
|---------|--------------|
| **SecBeat** | `/opt/secbeat/logs/` |
| **NATS** | `/var/log/nats/nats.log` |
| **HAProxy** | `/var/log/haproxy.log` |
| **System** | `journalctl -u service-name` |

## 🧪 Testing & Validation

### Automated Tests

```bash
# Run all deployment tests
./deploy.sh test

# Test specific components
ansible mitigation_nodes -i ansible/inventory.ini -m ping
ansible all -i ansible/inventory.ini -m shell -a "systemctl is-active --quiet secbeat-*"
```

### Manual Validation

```bash
# Test mitigation nodes
curl -k https://192.168.100.200:8443/health
curl -k https://192.168.100.201:8443/health
curl -k https://192.168.100.202:8443/health

# Test orchestrator API
curl http://192.168.100.203:9090/health

# Test monitoring
curl http://192.168.100.209:9090/-/healthy
curl http://192.168.100.209:3000/api/health

# Test load balancers
curl http://192.168.100.207:8404/stats
curl http://192.168.100.208:8404/stats
```

## 🛡️ Security Features

### Network Security
- **UFW Firewall**: Enabled on all VMs with specific rules
- **Fail2ban**: Protection against brute force attacks
- **SSH Key Authentication**: Password authentication disabled
- **TLS Encryption**: Auto-generated certificates for HTTPS

### Service Security
- **Non-root Services**: All services run as dedicated users
- **Resource Limits**: Memory and CPU limits for containers
- **Log Management**: Centralized logging with rotation
- **Regular Updates**: Automated security updates

## 🔄 Maintenance

### Updates

```bash
# Update SecBeat code
cd /opt/secbeat
git pull
cargo build --release
sudo systemctl restart secbeat-*

# Update system packages
ansible all -i ansible/inventory.ini -m apt -a "update_cache=yes upgrade=dist" --become
```

### Scaling

```bash
# Add more mitigation nodes (edit terraform/main.tf)
vim terraform/main.tf  # Increase mitigation_nodes.count

# Apply changes
./deploy.sh plan
./deploy.sh apply
./deploy.sh configure
```

### Backup

```bash
# Create VM snapshots
ssh root@192.168.100.23 "qm snapshot 200 backup-$(date +%Y%m%d)"

# Backup configuration
tar -czf secbeat-config-$(date +%Y%m%d).tar.gz deployment/
```

## 🚨 Troubleshooting

### Common Issues

#### Terraform Issues
```bash
# Provider initialization failed
terraform init -upgrade

# State lock issues
terraform force-unlock LOCK_ID

# VM creation timeout
# Increase timeout in main.tf or retry
```

#### Ansible Issues
```bash
# Connection timeout
ansible all -i ansible/inventory.ini -m ping --timeout=30

# SSH key issues
ssh-add ~/.ssh/id_rsa
ansible all -i ansible/inventory.ini -m ping -vvv
```

#### VM Boot Issues
```bash
# Check VM status on Proxmox
ssh root@192.168.100.23 "qm status 200-209"

# Check cloud-init logs
ssh secbeat@192.168.100.200 "sudo journalctl -u cloud-init"
```

### Debug Mode

```bash
# Enable verbose Terraform
TF_LOG=DEBUG ./deploy.sh apply

# Enable verbose Ansible
ANSIBLE_VERBOSITY=3 ./deploy.sh configure

# Enable script debugging
./deploy.sh deploy --verbose
```

## 🎯 Next Steps

After successful deployment:

1. **Configure SSL Certificates**: Replace self-signed certificates with CA-signed ones
2. **Setup Monitoring Alerts**: Configure Grafana dashboards and alerts
3. **Load Testing**: Run performance tests with your expected traffic
4. **Backup Strategy**: Implement regular backup procedures
5. **Security Hardening**: Apply additional security measures as needed

## 📝 File Structure

```
deployment/
├── deploy.sh                     # Main deployment script
├── terraform/
│   ├── main.tf                   # Terraform configuration
│   ├── terraform.tfvars.example  # Example variables
│   ├── cloud-init.yml.tpl        # Cloud-init template
│   └── inventory.ini.tpl          # Ansible inventory template
├── ansible/
│   ├── site.yml                  # Main playbook
│   ├── inventory.ini             # Generated by Terraform
│   └── templates/                # Service templates
│       ├── mitigation-node.service.j2
│       ├── orchestrator.service.j2
│       ├── nats.conf.j2
│       ├── haproxy.cfg.j2
│       ├── prometheus.yml.j2
│       └── docker-compose.yml.j2
└── README.md                     # This file
```

---

## 📞 Support

For issues and questions:
1. Check the troubleshooting section above
2. Review deployment logs in `/Users/fab/GitHub/secbeat/logs/deployment/`
3. Test individual components with the provided commands
4. File issues in the SecBeat repository with logs and configuration details
