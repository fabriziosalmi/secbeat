# SecBeat Kernel-Level Operations Guide

## Overview

SecBeat's SYN proxy functionality requires kernel-level network access for:
- Raw packet processing
- TCP/IP header manipulation  
- Direct network interface interaction
- Low-level socket operations

This guide explains how this works in our Proxmox VM deployment.

## 🔧 Kernel Requirements

### Required Capabilities
- `CAP_NET_RAW` - Create raw sockets
- `CAP_NET_ADMIN` - Network administration
- Access to `/dev/net/tun` (if using TUN/TAP)

### Required Packages
```bash
# Installed via cloud-init
libpcap-dev      # Raw packet capture library
build-essential  # Compilation tools
pkg-config       # Build configuration
libssl-dev       # TLS support
```

### Kernel Parameters
```bash
# Applied via /etc/sysctl.d/99-secbeat.conf
net.ipv4.ip_forward = 1                    # Enable IP forwarding
net.ipv4.conf.all.send_redirects = 0      # Security hardening
kernel.unprivileged_userns_clone = 1       # User namespace support
```

## 🚀 Deployment Architecture

### Why Proxmox VMs vs Kubernetes?

| Aspect | Proxmox VMs | Kubernetes |
|--------|-------------|------------|
| **Kernel Access** | ✅ Full access | ❌ Limited by container runtime |
| **Raw Sockets** | ✅ Native support | ❌ Requires privileged containers |
| **Network Stack** | ✅ Direct access | ❌ Abstracted by CNI |
| **Performance** | ✅ Minimal overhead | ❌ Additional network layers |
| **Security** | ✅ VM isolation | ❌ Container escape risks |

### VM Configuration

Each mitigation node VM gets:
```yaml
# Proxmox VM specs
cores: 1-2
memory: 1GB-2GB  
disk: 8GB-16GB
network: Direct bridge access (vmbr0)
```

## 🔐 Security Model

### Permission Escalation
```bash
# Via systemd service
ExecStartPre=/bin/bash -c 'sudo /sbin/setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node'

# Capabilities granted to binary
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node
```

### User Configuration
```bash
# secbeat user permissions
usermod -aG netdev secbeat                    # Network device group
echo "secbeat ALL=(ALL) NOPASSWD: /bin/setcap" >> /etc/sudoers.d/secbeat-caps
```

### Systemd Security
```ini
# Balanced security for kernel access
NoNewPrivileges=false                # Allow capability inheritance
ProtectKernelTunables=false         # Need packet processing access
RestrictRealtime=false              # May need RT for packet timing
AmbientCapabilities=CAP_NET_RAW CAP_NET_ADMIN
```

## 📊 Network Stack Integration

### Raw Packet Flow
```
1. Kernel receives packet on physical interface
2. Raw socket captures packet before normal stack
3. SecBeat processes TCP/IP headers
4. SYN cookie generated/validated
5. Modified packet re-injected or forwarded
```

### SYN Proxy Implementation
```rust
// Raw socket creation (requires CAP_NET_RAW)
let protocol = Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Tcp));
let (mut tx, mut rx) = transport_channel(4096, protocol)?;

// Packet processing loop
loop {
    match rx.next() {
        Ok((packet, addr)) => {
            if let Some(tcp_packet) = parse_tcp_packet(packet) {
                if tcp_packet.is_syn() {
                    handle_syn_with_cookie(tcp_packet).await?;
                }
            }
        }
    }
}
```

## 🛠️ Deployment Commands

### Proxmox Deployment
```bash
# Deploy full stack with kernel-level support
cd /Users/fab/GitHub/secbeat
./deploy_proxmox.sh test      # Validate environment
./deploy_proxmox.sh deploy    # Deploy VMs with proper permissions
```

### Manual Binary Deployment
```bash
# Build with kernel feature support
cargo build --release --features kernel-access

# Install with capabilities
sudo cp target/release/mitigation-node /usr/local/bin/
sudo setcap cap_net_raw,cap_net_admin+ep /usr/local/bin/mitigation-node

# Install systemd service
sudo cp systemd/secbeat-mitigation.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now secbeat-mitigation
```

### Verification
```bash
# Check capabilities
getcap /usr/local/bin/mitigation-node
# Should show: cap_net_admin,cap_net_raw+ep

# Check service status
sudo systemctl status secbeat-mitigation

# Test raw socket access
sudo -u secbeat /usr/local/bin/mitigation-node --test-caps
```

## 🔍 Troubleshooting

### Permission Issues
```bash
# Check user groups
groups secbeat

# Verify sudoers configuration
sudo visudo -c

# Check capability inheritance
systemctl show secbeat-mitigation | grep Capabilities
```

### Network Access Issues
```bash
# Check raw socket permissions
ls -la /dev/net/tun

# Verify sysctl parameters
sysctl net.ipv4.ip_forward
sysctl kernel.unprivileged_userns_clone

# Test packet capture
sudo tcpdump -i any port 8443
```

### Service Issues
```bash
# Check service logs
journalctl -u secbeat-mitigation -f

# Verify binary capabilities
getcap /usr/local/bin/mitigation-node

# Test manual execution
sudo -u secbeat /usr/local/bin/mitigation-node --config /etc/secbeat/mitigation.toml
```

## 🎯 Production Considerations

### Performance Optimization
- Use dedicated CPU cores for packet processing VMs
- Configure CPU affinity for network interrupts
- Tune kernel packet buffer sizes
- Consider DPDK for highest performance scenarios

### Security Hardening
- Regularly audit capability usage
- Monitor for privilege escalation attempts
- Use network segmentation between VMs
- Implement proper firewall rules

### Monitoring
- Track raw packet processing metrics
- Monitor capability usage
- Alert on permission changes
- Log all kernel-level operations

## 📈 Scaling Considerations

### Horizontal Scaling
```yaml
# Add more mitigation VMs
VM_CONFIGS[mitigation-4]="104:mitigation-node:2:2048:16:192.168.100.213"
VM_CONFIGS[mitigation-5]="105:mitigation-node:2:2048:16:192.168.100.214"
```

### Load Distribution
- Use consistent hashing for packet distribution
- Implement connection affinity
- Balance across multiple network interfaces
- Consider ECMP routing for traffic distribution

This architecture provides the kernel-level access needed for SecBeat's advanced DDoS protection while maintaining security and scalability through VM isolation.
