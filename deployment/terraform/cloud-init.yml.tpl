#cloud-config
# SecBeat VM Cloud-Init Configuration

hostname: ${hostname}
fqdn: ${hostname}.secbeat.local

# User configuration
users:
  - name: secbeat
    groups: sudo
    shell: /bin/bash
    sudo: ['ALL=(ALL) NOPASSWD:ALL']
    ssh_authorized_keys:
      - ${ssh_public_key}
  - name: root
    ssh_authorized_keys:
      - ${ssh_public_key}

# Package updates and installation
package_update: true
package_upgrade: true

packages:
  - curl
  - wget
  - git
  - htop
  - net-tools
  - unzip
  - jq
  - fail2ban
  - ufw
  - docker.io
  - docker-compose
  - prometheus-node-exporter
  - rsyslog

# System configuration
timezone: UTC
locale: en_US.UTF-8

# Network configuration
write_files:
  # SecBeat TLS certificate
  - path: /etc/ssl/certs/secbeat.pem
    content: ${secbeat_cert}
    encoding: base64
    permissions: '0644'
    
  # SecBeat TLS private key
  - path: /etc/ssl/private/secbeat.key
    content: ${secbeat_key}
    encoding: base64
    permissions: '0600'
    
  # Docker daemon configuration
  - path: /etc/docker/daemon.json
    content: |
      {
        "log-driver": "json-file",
        "log-opts": {
          "max-size": "10m",
          "max-file": "3"
        },
        "storage-driver": "overlay2"
      }
    permissions: '0644'
    
  # Fail2ban configuration for SecBeat
  - path: /etc/fail2ban/jail.local
    content: |
      [DEFAULT]
      bantime = 3600
      findtime = 600
      maxretry = 5
      
      [sshd]
      enabled = true
      port = ssh
      logpath = /var/log/auth.log
      
      [secbeat]
      enabled = true
      port = 8443
      protocol = tcp
      filter = secbeat
      logpath = /var/log/secbeat/mitigation-node.log
      maxretry = 10
      bantime = 7200
    permissions: '0644'
    
  # UFW firewall rules
  - path: /etc/ufw/applications.d/secbeat
    content: |
      [SecBeat]
      title=SecBeat Security Platform
      description=SecBeat DDoS mitigation and WAF platform
      ports=8443/tcp
      
      [SecBeatManagement]
      title=SecBeat Management API
      description=SecBeat management and monitoring
      ports=9191,9192/tcp
      
      [SecBeatOrchestrator]
      title=SecBeat Orchestrator
      description=SecBeat orchestrator API
      ports=9090/tcp
    permissions: '0644'
    
  # System limits for high performance
  - path: /etc/security/limits.d/99-secbeat.conf
    content: |
      # SecBeat performance limits
      secbeat soft nofile 65536
      secbeat hard nofile 65536
      secbeat soft nproc 32768
      secbeat hard nproc 32768
      * soft nofile 65536
      * hard nofile 65536
    permissions: '0644'
    
  # Kernel tuning for network performance
  - path: /etc/sysctl.d/99-secbeat.conf
    content: |
      # Network performance tuning for SecBeat
      net.core.rmem_max = 134217728
      net.core.wmem_max = 134217728
      net.ipv4.tcp_rmem = 4096 87380 134217728
      net.ipv4.tcp_wmem = 4096 65536 134217728
      net.core.netdev_max_backlog = 5000
      net.core.somaxconn = 1024
      net.ipv4.tcp_max_syn_backlog = 8192
      net.ipv4.tcp_slow_start_after_idle = 0
      net.ipv4.tcp_tw_reuse = 1
      net.ipv4.ip_local_port_range = 1024 65535
      fs.file-max = 2097152
      vm.swappiness = 10
    permissions: '0644'

# Service configuration
runcmd:
  # System setup
  - systemctl enable docker
  - systemctl start docker
  - usermod -aG docker secbeat
  
  # Apply kernel tuning
  - sysctl -p /etc/sysctl.d/99-secbeat.conf
  
  # Configure firewall
  - ufw --force enable
  - ufw default deny incoming
  - ufw default allow outgoing
  - ufw allow ssh
  - ufw allow SecBeat
  - ufw allow from 192.168.200.0/24 to any port 9191 # Metrics
  - ufw allow from 192.168.200.0/24 to any port 9192 # Management
  - ufw allow from 192.168.200.0/24 to any port 9090 # Orchestrator
  - ufw allow from 192.168.200.0/24 to any port 4222 # NATS
  
  # Start and enable services
  - systemctl enable fail2ban
  - systemctl start fail2ban
  - systemctl enable prometheus-node-exporter
  - systemctl start prometheus-node-exporter
  
  # Create SecBeat directories
  - mkdir -p /opt/secbeat/{bin,config,logs,data}
  - chown -R secbeat:secbeat /opt/secbeat
  - mkdir -p /var/log/secbeat
  - chown secbeat:secbeat /var/log/secbeat
  
  # Download and install Rust (for building from source if needed)
  - su - secbeat -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
  - su - secbeat -c "echo 'source ~/.cargo/env' >> ~/.bashrc"
  
  # Set up log rotation
  - |
    cat > /etc/logrotate.d/secbeat << EOF
    /var/log/secbeat/*.log {
        daily
        missingok
        rotate 30
        compress
        delaycompress
        notifempty
        copytruncate
        su secbeat secbeat
    }
    EOF

# Final message
final_message: |
  SecBeat VM initialization complete!
  
  VM Details:
  - Hostname: ${hostname}.secbeat.local
  - User: secbeat (with sudo access)
  - SSH: Key-based authentication configured
  - Docker: Installed and configured
  - Firewall: UFW enabled with SecBeat rules
  - Monitoring: Node exporter running on port 9100
  - Logs: /var/log/secbeat/
  - SecBeat Directory: /opt/secbeat/
  
  Next Steps:
  1. Deploy SecBeat binaries to /opt/secbeat/bin/
  2. Configure SecBeat with appropriate config files
  3. Set up systemd services for SecBeat components
  4. Test connectivity and functionality
  
  For support: https://github.com/your-org/secbeat

# Reboot after initialization
power_state:
  mode: reboot
  message: "Rebooting after SecBeat VM initialization"
  timeout: 30
  condition: true
