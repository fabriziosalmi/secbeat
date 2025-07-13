# SecBeat Proxmox Deployment Checklist

## Pre-Deployment Setup

### Proxmox Environment
- [ ] Proxmox VE 7.0+ installed and accessible at 192.168.100.23
- [ ] Ubuntu 24.04.2 LTS ISO uploaded to Proxmox storage
- [ ] ISO available at `/var/lib/vz/template/iso/ubuntu-24.04.2-live-server-amd64.iso`
- [ ] Sufficient resources available (8+ cores, 8+ GB RAM, 100+ GB storage)
- [ ] Network bridges configured (vmbr0 recommended)
- [ ] Internet connectivity available for package downloads

### Local Environment
- [ ] SSH key pair generated (`ssh-keygen -t rsa -b 4096 -f ~/.ssh/id_rsa`)
- [ ] SSH public key copied to Proxmox host (`ssh-copy-id root@192.168.100.23`)
- [ ] SSH connectivity tested (`ssh root@192.168.100.23`)
- [ ] SecBeat project directory is current working directory

### Configuration Files
- [ ] Proxmox host IP updated in `deployment/proxmox-config.yml`
- [ ] Network settings reviewed in `deployment/proxmox-config.yml`
- [ ] Production config reviewed in `config/production.toml`
- [ ] SSL certificates prepared (optional for initial deployment)

## Deployment Process

### Phase 1: Pre-Deployment Testing
```bash
./deploy_proxmox.sh test
```
- [ ] SSH connectivity to Proxmox verified
- [ ] ISO availability confirmed
- [ ] Storage space validated
- [ ] Network bridge accessibility checked
- [ ] Test VM created successfully
- [ ] Cloud-init configuration working
- [ ] Package installation tested
- [ ] SecBeat code compilation verified

### Phase 2: Full Deployment
```bash
./deploy_proxmox.sh deploy
```
- [ ] All 10 VMs created successfully
- [ ] All VMs started and accessible via SSH
- [ ] Base software installed on all VMs
- [ ] SecBeat code deployed and compiled
- [ ] Services configured and started
- [ ] Load balancers configured
- [ ] Monitoring stack deployed

### Phase 3: Verification
```bash
./deploy_proxmox.sh status
```
- [ ] All VMs responding to SSH
- [ ] SecBeat mitigation services running
- [ ] NATS cluster operational
- [ ] Load balancers responding
- [ ] Prometheus collecting metrics
- [ ] Grafana dashboard accessible

## Post-Deployment Validation

### Service Health Checks
- [ ] Mitigation nodes responding on ports 8443
- [ ] Orchestrator API accessible on port 9090
- [ ] NATS cluster healthy on ports 4222
- [ ] HAProxy stats accessible on port 8404
- [ ] Prometheus metrics on port 9090
- [ ] Grafana dashboard on port 3000

### Access Verification
- [ ] SSH access to all VMs using `secbeat` user
- [ ] Sudo privileges working on all VMs
- [ ] Service logs accessible via journalctl
- [ ] Grafana login working (admin/secbeat123)

### Network Connectivity
- [ ] Inter-VM communication working
- [ ] Internet access from VMs
- [ ] Load balancer routing traffic correctly
- [ ] Monitoring collecting data from all nodes

### Performance Baseline
- [ ] Basic load test passed
- [ ] Response times within acceptable limits
- [ ] Resource usage at normal levels
- [ ] No critical alerts in monitoring

## Access Information

| Service | URL | Credentials |
|---------|-----|-------------|
| Grafana | http://192.168.100.209:3000 | admin/secbeat123 |
| Prometheus | http://192.168.100.209:9090 | N/A |
| HAProxy Stats | http://192.168.100.207:8404/stats | N/A |

| VM Type | IP Addresses | RAM | Disk | SSH Access |
|---------|--------------|-----|------|------------|
| Mitigation Nodes | 192.168.100.200-202 | 1GB each | 8GB each | ssh -i ~/.ssh/id_rsa secbeat@IP |
| Orchestrator | 192.168.100.203 | 1GB | 8GB | ssh -i ~/.ssh/id_rsa secbeat@IP |
| NATS Cluster | 192.168.100.204-206 | 512MB each | 6GB each | ssh -i ~/.ssh/id_rsa secbeat@IP |
| Load Balancers | 192.168.100.207-208 | 512MB each | 6GB each | ssh -i ~/.ssh/id_rsa secbeat@IP |
| Monitoring | 192.168.100.209 | 1.5GB | 12GB | ssh -i ~/.ssh/id_rsa secbeat@IP |

**Total Resources: 7.5GB RAM, 80GB Disk**
**Network: Single 192.168.100.0/24 subnet (IPs 200-220 range)**

## Troubleshooting Checklist

### If SSH Connection Fails
- [ ] Verify SSH key permissions (`chmod 600 ~/.ssh/id_rsa`)
- [ ] Check Proxmox host accessibility
- [ ] Confirm public key is in authorized_keys

### If VM Creation Fails
- [ ] Check Proxmox storage space
- [ ] Verify ISO is uploaded correctly
- [ ] Confirm VM ID is not already in use
- [ ] Check network bridge configuration

### If Service Won't Start
- [ ] Check service logs with journalctl
- [ ] Verify configuration file syntax
- [ ] Check port availability
- [ ] Confirm dependencies are installed

### If Monitoring Not Working
- [ ] Check Docker service status
- [ ] Verify Docker Compose file syntax
- [ ] Check container logs
- [ ] Confirm network connectivity

## Cleanup Procedure

If deployment fails and you need to start over:

```bash
# Remove all VMs and start fresh
./deploy_proxmox.sh cleanup

# Wait for cleanup to complete, then redeploy
./deploy_proxmox.sh deploy
```

## Next Steps After Successful Deployment

### Security Hardening
- [ ] Install SSL certificates
- [ ] Configure firewall rules
- [ ] Set up fail2ban
- [ ] Change default passwords
- [ ] Enable audit logging

### Monitoring Setup
- [ ] Configure Grafana dashboards
- [ ] Set up alerting rules
- [ ] Configure notification channels
- [ ] Define SLA metrics

### Operational Procedures
- [ ] Document maintenance procedures
- [ ] Set up backup schedules
- [ ] Create incident response plan
- [ ] Train operations team

### Performance Optimization
- [ ] Run comprehensive load tests
- [ ] Tune system parameters
- [ ] Optimize resource allocation
- [ ] Set up capacity monitoring

---

**Deployment Time Estimate**: 45-90 minutes depending on hardware
**Support**: Check deployment/README.md for detailed troubleshooting
