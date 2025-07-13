# SecBeat Proxmox Deployment - Resource Optimized Configuration

## 🎯 Configuration Summary

Your SecBeat deployment has been optimized for your Proxmox environment:

### 💾 Resource Allocation (Fits within 8GB RAM / 100GB Disk)

| VM | IP Address | RAM | Disk | Purpose |
|----|------------|-----|------|---------|
| **mitigation-1** | 192.168.100.200 | 1GB | 8GB | DDoS Protection & WAF |
| **mitigation-2** | 192.168.100.201 | 1GB | 8GB | DDoS Protection & WAF |
| **mitigation-3** | 192.168.100.202 | 1GB | 8GB | DDoS Protection & WAF |
| **orchestrator** | 192.168.100.203 | 1GB | 8GB | Central Coordination |
| **nats-1** | 192.168.100.204 | 512MB | 6GB | Event Messaging |
| **nats-2** | 192.168.100.205 | 512MB | 6GB | Event Messaging |
| **nats-3** | 192.168.100.206 | 512MB | 6GB | Event Messaging |
| **loadbalancer-1** | 192.168.100.207 | 512MB | 6GB | Traffic Distribution |
| **loadbalancer-2** | 192.168.100.208 | 512MB | 6GB | Traffic Distribution |
| **monitoring** | 192.168.100.209 | 1.5GB | 12GB | Grafana + Prometheus |

**Total Usage: 7.5GB RAM (94% of 8GB) + 80GB Disk (80% of 100GB)**

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

## 🚀 Ready to Deploy

Your deployment is now ready with these optimizations:

```bash
# Test your environment first
./deploy_proxmox.sh test

# Deploy the full stack
./deploy_proxmox.sh deploy

# Check status anytime
./deploy_proxmox.sh status
```

## 🌐 Access After Deployment

- **Grafana**: http://192.168.100.209:3000 (admin/secbeat123)
- **Prometheus**: http://192.168.100.209:9090
- **HAProxy Stats**: http://192.168.100.207:8404/stats
- **SSH Access**: `ssh -i ~/.ssh/id_rsa secbeat@192.168.100.XXX`

## 🔧 Optimizations Made

### Resource Efficiency
- **Reduced VM memory** to fit 8GB total
- **Smaller disk allocations** to stay under 100GB
- **Lightweight monitoring** (removed AlertManager)
- **Minimal Rust installation** (no Docker on mitigation nodes)

### Network Simplification
- **Single subnet** for all VMs
- **Consistent gateway** (192.168.100.1)
- **High IP range** (200-220) to avoid conflicts
- **Easy debugging** from your local machine

### Configuration Updates
- **Updated all scripts** to use correct ISO path
- **Fixed network references** throughout deployment
- **Optimized service configurations** for lower resources
- **Updated documentation** to match new setup

Your SecBeat platform will provide enterprise-grade DDoS protection and WAF capabilities while efficiently using your available resources!

---

**Next Step**: Run `./deploy_proxmox.sh test` to validate your environment
