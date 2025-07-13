# SecBeat Proxmox Deployment Guide

This directory contains the complete automation for deploying SecBeat to a Proxmox Virtual Environment (PVE) cluster.

## Overview

The deployment system provides:
- **Automated VM provisioning** with cloud-init
- **Multi-node architecture** with load balancing
- **Complete software stack** installation (Rust, NATS, monitoring)
- **Production-ready configuration** 
- **Health checks and monitoring**
- **Comprehensive logging and reporting**

## Architecture

### Deployed Components

| Component | VMs | Purpose | IP Range |
|-----------|-----|---------|----------|
| Mitigation Nodes | 3 | DDoS protection and WAF | 192.168.100.200-202 |
| Orchestrator | 1 | Central coordination | 192.168.100.203 |
| NATS Cluster | 3 | Event messaging | 192.168.100.204-206 |
| Load Balancers | 2 | Traffic distribution | 192.168.100.207-208 |
| Monitoring | 1 | Grafana/Prometheus | 192.168.100.209 |

### Network Configuration

- **Management Network**: 192.168.100.0/24
- **Application Network**: 192.168.100.0/24 (same network for easier debugging)  
- **VM IP Range**: 192.168.100.200-220 (avoids lower addresses in use)

## Prerequisites

### Proxmox Host Requirements

1. **Proxmox VE 7.0+** installed and configured
2. **Ubuntu 24.04.2 LTS ISO** uploaded to Proxmox storage
3. **SSH access** configured with key-based authentication
4. **Internet connectivity** for package downloads
5. **Sufficient resources** (optimized for smaller environments):
   - CPU: 8+ cores
   - RAM: 8+ GB
   - Storage: 100+ GB

### Local Requirements

1. **SSH key pair** at `~/.ssh/id_rsa`
2. **Network connectivity** to Proxmox host
3. **Bash 4.0+** shell environment

## Quick Start

### 1. Configure Connection

Update the Proxmox host IP in configuration files:

```bash
# Edit deployment configuration
nano deployment/proxmox-config.yml

# Update host IP (default: 192.168.100.23)
proxmox:
  node: "YOUR_PROXMOX_HOST_IP"
```

### 2. Setup SSH Access

```bash
# Generate SSH key if needed
ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa

# Copy public key to Proxmox host
ssh-copy-id root@YOUR_PROXMOX_HOST_IP
```

### 3. Upload Ubuntu ISO

1. Access Proxmox web interface
2. Go to Datacenter → Storage → local → ISO Images
3. Upload `ubuntu-22.04-server-amd64.iso`

### 4. Run Deployment

```bash
# Test connection and environment
./deploy_proxmox.sh test

# Run full deployment
./deploy_proxmox.sh deploy
```

## Deployment Scripts

### Main Orchestrator

```bash
./deploy_proxmox.sh [COMMAND]
```

**Commands:**
- `test` - Run pre-deployment validation
- `deploy` - Execute full deployment
- `status` - Check deployment health
- `cleanup` - Remove all deployed VMs
- `help` - Show usage information

### Component Scripts

| Script | Purpose |
|--------|---------|
| `deployment/scripts/test-proxmox.sh` | Pre-deployment testing and validation |
| `deployment/scripts/deploy-proxmox.sh` | Full multi-VM deployment automation |

## Configuration Files

### Primary Configuration

- **`proxmox-config.yml`** - Main deployment configuration
- **`config/production.toml`** - SecBeat runtime configuration
- **`config/staging.toml`** - Staging environment settings

### Auto-Generated

- **Cloud-init configs** - VM initialization scripts
- **Service configs** - Systemd service definitions
- **Network configs** - Networking and firewall rules

## Deployment Process

### Phase 1: Pre-Deployment Tests

1. **Connectivity verification** - SSH access to Proxmox
2. **Resource validation** - ISO availability, storage space
3. **Network checks** - Bridge configuration, IP ranges
4. **Test VM creation** - Single VM deployment test

### Phase 2: VM Provisioning

1. **VM creation** - Allocate resources, attach storage
2. **Cloud-init setup** - Configure users, network, packages
3. **VM startup** - Boot VMs and wait for SSH access
4. **Basic validation** - Test connectivity and sudo access

### Phase 3: Software Installation

1. **Base packages** - System dependencies and tools
2. **Runtime environments** - Rust, Docker, NATS
3. **SecBeat deployment** - Source code and compilation
4. **Service configuration** - Systemd services and configs

### Phase 4: Service Configuration

1. **Mitigation nodes** - DDoS protection services
2. **Orchestrator** - Central coordination service
3. **NATS cluster** - Event messaging system
4. **Load balancers** - HAProxy and Nginx setup
5. **Monitoring stack** - Prometheus and Grafana

### Phase 5: Validation & Reporting

1. **Health checks** - Service status verification
2. **Connectivity tests** - End-to-end validation
3. **Performance tests** - Basic load testing
4. **Report generation** - Deployment summary and access info

## Monitoring and Management

### Access URLs

- **Grafana Dashboard**: http://192.168.100.209:3000
- **Prometheus Metrics**: http://192.168.100.209:9090
- **HAProxy Stats**: http://192.168.100.207:8404/stats

### Default Credentials

- **Grafana**: admin / secbeat123
- **System SSH**: secbeat user with sudo access

### Log Locations

```bash
# Deployment logs
logs/deployment/

# Service logs on VMs
journalctl -u secbeat-mitigation -f
journalctl -u secbeat-orchestrator -f
```

## Troubleshooting

### Common Issues

#### SSH Connection Failed
```bash
# Check SSH key permissions
chmod 600 ~/.ssh/id_rsa
chmod 644 ~/.ssh/id_rsa.pub

# Test SSH connectivity
ssh -i ~/.ssh/id_rsa root@PROXMOX_HOST
```

#### ISO Not Found
```bash
# Verify ISO upload in Proxmox web UI
# Expected location: /var/lib/vz/template/iso/
```

#### VM Creation Failed
```bash
# Check Proxmox storage space
ssh root@PROXMOX_HOST "df -h"

# Check available VM IDs
ssh root@PROXMOX_HOST "qm list"
```

#### Service Won't Start
```bash
# Check service logs
ssh secbeat@VM_IP "sudo journalctl -u SERVICE_NAME -f"

# Check configuration
ssh secbeat@VM_IP "sudo systemctl status SERVICE_NAME"
```

### Log Analysis

```bash
# View deployment progress
tail -f logs/deployment/deployment_*.log

# Check specific VM deployment
grep "VM_NAME" logs/deployment/deployment_*.log

# View test results
cat logs/deployment/deployment_report_*.md
```

### Manual Recovery

#### Restart Failed Service
```bash
ssh secbeat@VM_IP "sudo systemctl restart secbeat-mitigation"
```

#### Rebuild Failed VM
```bash
# Destroy and recreate specific VM
ssh root@PROXMOX_HOST "qm stop VM_ID && qm destroy VM_ID"

# Re-run deployment (will skip existing VMs)
./deploy_proxmox.sh deploy
```

#### Clean Start
```bash
# Remove all VMs and start fresh
./deploy_proxmox.sh cleanup
./deploy_proxmox.sh deploy
```

## Advanced Configuration

### Custom VM Resources

Edit `deployment/scripts/deploy-proxmox.sh`:

```bash
# VM Configuration (format: id:type:cores:memory:disk:ip)
VM_CONFIGS[mitigation-1]="101:mitigation-node:8:16384:80:192.168.200.10"
```

### Custom Network Configuration

Edit `deployment/proxmox-config.yml`:

```yaml
network:
  subnets:
    management: "10.0.1.0/24"
    application: "10.0.2.0/24"
    monitoring: "10.0.3.0/24"
```

### Additional Monitoring

```bash
# Add custom exporters
ssh secbeat@192.168.300.10 "cd /opt/monitoring && vim docker-compose.yml"
```

## Security Considerations

### Network Security

- Firewall rules restrict access to management interfaces
- TLS encryption for all external communications
- Fail2ban protection against brute force attacks

### Access Control

- SSH key-based authentication only
- Dedicated service accounts with minimal privileges
- Regular security updates via automated patching

### Data Protection

- Encrypted communications between services
- Secure credential storage and rotation
- Regular backup procedures (configure separately)

## Production Deployment

### Pre-Production Checklist

- [ ] SSL certificates installed and configured
- [ ] Monitoring dashboards configured
- [ ] Alerting rules defined and tested
- [ ] Backup procedures implemented
- [ ] Security hardening applied
- [ ] Load testing completed
- [ ] Disaster recovery plan documented

### Performance Tuning

1. **VM Resources** - Allocate based on expected load
2. **Network Optimization** - Tune TCP/IP stack parameters
3. **Storage Performance** - Use SSD storage for databases
4. **Monitoring Retention** - Configure appropriate data retention

### Maintenance

- **Updates**: Use `cargo update` for Rust dependencies
- **Monitoring**: Monitor resource usage and performance
- **Backups**: Implement automated VM snapshots
- **Security**: Apply security patches regularly

## Support

### Getting Help

1. **Check logs** in `logs/deployment/` directory
2. **Review configuration** files for typos
3. **Test connectivity** to Proxmox host
4. **Verify prerequisites** are met

### Reporting Issues

When reporting issues, include:
- Deployment command used
- Error messages from logs
- Proxmox host specifications
- Network configuration details

---

**Version**: 1.0.0  
**Last Updated**: December 2024  
**Compatibility**: Proxmox VE 7.0+, Ubuntu 22.04 LTS
