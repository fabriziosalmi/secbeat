# SecBeat Proxmox Deployment - Resource Optimized Configuration

## 🎯 Configuration Summary

Your SecBeat deployment has been optimized for your Proxmox environment:

### 💾 Resource Allocation (Fits within 8GB RAM / 100GB Disk)

| VM | IP Address | RAM | Disk | Purpose |
|----|------------|-----|------|---------|
| **mitigation-1** | 192.168.100.100 | 2GB | 20GB | DDoS Protection & WAF |
| **mitigation-2** | 192.168.100.101 | 2GB | 20GB | DDoS Protection & WAF |
| **mitigation-3** | 192.168.100.102 | 2GB | 20GB | DDoS Protection & WAF |
| **orchestrator** | 192.168.100.103 | 4GB | 20GB | Central Coordination |
| **nats-1** | 192.168.100.104 | 2GB | 20GB | Event Messaging |
| **nats-2** | 192.168.100.105 | 2GB | 20GB | Event Messaging |
| **nats-3** | 192.168.100.106 | 2GB | 20GB | Event Messaging |
| **loadbalancer-1** | 192.168.100.107 | 2GB | 20GB | Traffic Distribution |
| **loadbalancer-2** | 192.168.100.108 | 2GB | 20GB | Traffic Distribution |
| **monitoring** | 192.168.100.109 | 4GB | 20GB | Grafana + Prometheus |


### 🌐 Network Configuration

- **Single Network**: 192.168.100.0/24
- **VM IP Range**: 192.168.100.200-220
- **Benefits**:
  - Easy debugging from your macOS client (same subnet)
  - No routing complexity
  - Direct access to all services
  - Avoids IP conflicts (200+ range)

### 💿 ISO Configuration

- **ISO Path**: `/var/lib/vz/template/iso/ubuntu-24.04.2-live-server-amd64.iso`
- **Verified**: Scripts updated to use your exact ISO path

## 🌐 Access After Deployment

- **Grafana**: http://192.168.100.209:3000 (admin/secbeat123)
- **Prometheus**: http://192.168.100.209:9090
- **HAProxy Stats**: http://192.168.100.207:8404/stats
- **SSH Access**: `ssh -i ~/.ssh/id_rsa secbeat@192.168.100.XXX`

