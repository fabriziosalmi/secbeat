# SecBeat VM Allocation Plan

## Protected LXC Containers (DO NOT TOUCH)
- **LXC 202**: IP 192.168.100.236 (protected)
- **LXC 204**: IP 192.168.100.240 (protected)

## SecBeat VM Allocation (Using Available Range 100-120)

### VM IDs and IP Addresses
| Component | VM ID | IP Address | Node Name |
|-----------|-------|------------|-----------|
| Mitigation Node 1 | 100 | 192.168.100.100 | secbeat-mitigation-1 |
| Mitigation Node 2 | 101 | 192.168.100.101 | secbeat-mitigation-2 |
| Mitigation Node 3 | 102 | 192.168.100.102 | secbeat-mitigation-3 |
| Orchestrator | 110 | 192.168.100.103 | secbeat-orchestrator |
| NATS Node 1 | 111 | 192.168.100.104 | secbeat-nats-1 |
| NATS Node 2 | 112 | 192.168.100.105 | secbeat-nats-2 |
| NATS Node 3 | 113 | 192.168.100.106 | secbeat-nats-3 |
| Load Balancer 1 | 114 | 192.168.100.107 | secbeat-lb-1 |
| Load Balancer 2 | 115 | 192.168.100.108 | secbeat-lb-2 |
| Monitoring | 116 | 192.168.100.109 | secbeat-monitoring |

### Resource Allocation (Optimized for 16GB System)
| Component | vCPUs | RAM (MB) | Disk (GB) | Count |
|-----------|-------|----------|-----------|-------|
| Mitigation Nodes | 1 | 768 | 8 | 3 |
| Orchestrator | 1 | 768 | 8 | 1 |
| NATS Cluster | 1 | 512 | 6 | 3 |
| Load Balancers | 1 | 768 | 6 | 2 |
| Monitoring | 1 | 1536 | 12 | 1 |

**Total Resources**: 10 vCPUs, 7.8GB RAM, 74GB Disk

### Memory Safety Analysis
| Item | Memory Usage |
|------|--------------|
| **Existing LXCs** | 3GB (2GB + 1GB) |
| **Host OS + overhead** | 1.5GB |
| **SecBeat VMs** | 7.8GB |
| **Total Used** | 12.3GB |
| **Available from 16GB** | 3.7GB free |
| **Safety Margin** | ✅ 23% free (safe for production) |

### Network Configuration
- **Network**: 192.168.100.0/24
- **Gateway**: 192.168.100.1
- **DNS**: 8.8.8.8, 8.8.4.4
- **Bridge**: vmbr0

### Deployment Safety
✅ **No VM ID conflicts** with existing LXC containers 202, 204
✅ **No IP address conflicts** with protected IPs 192.168.100.236, 192.168.100.240
✅ **IP range 192.168.100.100-109** is cleanly allocated for SecBeat
✅ **VM ID range 100-116** uses available address space efficiently
✅ **Reserved space**: IPs 110-120 available for future expansion

### Key Features
- **Kernel-level capabilities**: All VMs configured with CAP_NET_RAW and CAP_NET_ADMIN
- **Raw socket support**: Network tuning for packet processing
- **Systemd hardening**: Security and resource limits
- **Cloud-init automation**: Automated setup and configuration
- **TLS certificates**: Self-signed certificates for secure communication
- **Monitoring stack**: Prometheus + Grafana for observability

## Commands for Deployment
```bash
# Test Terraform plan
cd deployment/terraform
terraform plan

# Deploy infrastructure
terraform apply

# Check deployment status
cd ../..
./deploy_proxmox.sh status
```

## Emergency Rollback
If any issues occur with VM creation, use:
```bash
cd deployment/terraform
terraform destroy -target=proxmox_vm_qemu.mitigation_nodes
terraform destroy -target=proxmox_vm_qemu.orchestrator
# etc.
```

This plan ensures complete isolation from your existing LXC containers while providing a robust SecBeat deployment infrastructure.
