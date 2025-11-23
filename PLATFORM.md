# SecBeat Platform Guide

This document provides comprehensive information about the SecBeat DDoS mitigation and WAF platform, including its architecture, capabilities, deployment strategies, and operational procedures.

## Table of Contents

- [Platform Architecture](#platform-architecture)
- [Core Capabilities](#core-capabilities)
- [Operation Modes](#operation-modes)
- [Deployment Strategies](#deployment-strategies)
- [Configuration Management](#configuration-management)
- [Monitoring and Observability](#monitoring-and-observability)
- [Security Features](#security-features)
- [Performance Characteristics](#performance-characteristics)
- [Operational Procedures](#operational-procedures)
- [Troubleshooting Guide](#troubleshooting-guide)

## Platform Architecture

SecBeat implements a distributed "smart edge, intelligent orchestrator" architecture that provides both high-performance traffic processing and centralized intelligence.

### System Components

**Mitigation Nodes (Edge Layer)**
- High-performance traffic processing engines
- Multiple operation modes: TCP, SYN, Layer 7
- Real-time threat detection and mitigation
- Local decision-making capabilities
- Horizontal scaling support

**Orchestrator Node (Control Plane)**
- Centralized fleet management and coordination
- AI-powered decision engine and threat intelligence
- Resource optimization and predictive scaling
- Policy distribution and management
- Global situational awareness

**Communication Layer**
- NATS-based real-time messaging
- RESTful APIs for management operations
- Webhook integration for automation
- Metrics and telemetry streaming

### Data Flow Architecture

```
Internet Traffic → Mitigation Nodes → Backend Services
                        ↕
                  Orchestrator Node
                  (Control & Intelligence)
```

## Core Capabilities

### Multi-Layer Protection

**Layer 4 (Network/Transport)**
- TCP/UDP proxy with sub-millisecond latency
- SYN flood protection using kernel-level packet processing
- Connection rate limiting and state tracking
- Network-level DDoS mitigation

**Layer 7 (Application)**
- HTTPS termination with modern TLS support
- Web Application Firewall with dynamic rules
- HTTP/2 protocol support
- Request filtering and content inspection

### AI-Powered Intelligence

**Threat Detection**
- Real-time attack pattern recognition
- Behavioral analysis and anomaly detection
- Cross-correlation of security events
- Machine learning-based threat modeling

**Autonomous Response**
- Dynamic rule generation and deployment
- Automated scaling based on traffic patterns
- Self-healing node replacement
- Intelligent load balancing

### Platform Features

**High Availability**
- Distributed architecture with no single points of failure
- Automatic failover and recovery
- Graceful degradation under load
- Health monitoring and self-healing

**Scalability**
- Horizontal scaling of mitigation nodes
- Predictive scaling based on ML models
- Dynamic resource allocation
- Cloud-agnostic deployment

## Operation Modes

SecBeat mitigation nodes support three primary operation modes, each optimized for specific security and performance requirements.

### TCP Mode

**Purpose**: Basic high-performance TCP proxy
**Use Cases**: Load balancing, simple traffic forwarding
**Security Level**: Basic
**Performance**: Highest throughput, lowest latency

**Features**:
- Asynchronous bidirectional TCP forwarding
- Connection pooling and reuse
- Basic connection rate limiting
- Minimal processing overhead

**Configuration**:
```toml
[server]
mode = "tcp"
bind_address = "0.0.0.0:8443"
backend_address = "127.0.0.1:8080"
```

### SYN Mode

**Purpose**: SYN flood protection and DDoS mitigation
**Use Cases**: High-volume DDoS protection, attack resilience
**Security Level**: High
**Performance**: High throughput with security overhead

**Features**:
- SYN cookie validation
- Raw packet processing
- Connection state tracking
- Automatic attack detection and blocking

**Requirements**:
- Root privileges for raw socket access
- Kernel-level packet interception capability

**Configuration**:
```toml
[server]
mode = "syn"
bind_address = "0.0.0.0:8443"
backend_address = "127.0.0.1:8080"

[syn_proxy]
enable = true
max_syn_backlog = 65536
syn_cookie_secret = "your-secret-key"
```

### Layer 7 Mode

**Purpose**: Full application-layer processing with WAF
**Use Cases**: Web application protection, content filtering
**Security Level**: Maximum
**Performance**: Moderate with full security features

**Features**:
- HTTPS termination and certificate management
- Web Application Firewall with dynamic rules
- HTTP/2 support
- Content inspection and filtering
- Attack signature detection

**Configuration**:
```toml
[server]
mode = "l7"
bind_address = "0.0.0.0:8443"
backend_address = "127.0.0.1:8080"

[tls]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"

[waf]
enable = true
rules_path = "config/waf_rules.json"
```

## Deployment Strategies

### Single Node Deployment

**Use Case**: Development, testing, small-scale production
**Architecture**: Single mitigation node with optional orchestrator
**Capacity**: Up to 50K RPS, 10K concurrent connections

```bash
# Basic single-node deployment
cd mitigation-node
export MITIGATION_CONFIG=config/production.toml
sudo cargo run --release
```

### Multi-Node Cluster

**Use Case**: High-availability production environments
**Architecture**: Multiple mitigation nodes with orchestrator coordination
**Capacity**: 500K+ RPS, 100K+ concurrent connections

```bash
# Deploy orchestrator
cd orchestrator-node
cargo run --release &

# Deploy multiple mitigation nodes
for i in {1..5}; do
    cd mitigation-node
    export MITIGATION_CONFIG=config/node-$i.toml
    sudo cargo run --release &
done
```

### Cloud Deployment

**Container Deployment**:
```bash
# Build and deploy with Docker
make docker-build
docker-compose up -d
```

**Kubernetes Deployment**:
```bash
# Deploy to Kubernetes cluster
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
```

**Terraform Deployment**:
```bash
# Deploy infrastructure with Terraform
cd terraform/aws
terraform init
terraform plan
terraform apply
```

### Hybrid Deployment

**Use Case**: Multi-cloud and on-premises integration
**Architecture**: Distributed nodes across multiple environments
**Benefits**: Geographic distribution, vendor independence

## Configuration Management

### Configuration Hierarchy

1. **Environment Variables**: Runtime overrides
2. **Configuration Files**: Persistent settings
3. **Command Line Arguments**: One-time overrides
4. **Default Values**: Built-in fallbacks

### Configuration Files

**Production Configuration** (`config/production.toml`):
```toml
[server]
mode = "l7"
bind_address = "0.0.0.0:8443"
backend_address = "127.0.0.1:8080"
worker_threads = 0  # Auto-detect CPU cores

[tls]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
protocols = ["TLSv1.3", "TLSv1.2"]

[syn_proxy]
enable = true
max_syn_backlog = 65536
syn_cookie_secret = "${SYN_COOKIE_SECRET}"

[waf]
enable = true
rules_path = "config/waf_rules.json"
block_suspicious = true
rate_limit_rps = 1000

[orchestrator]
url = "http://orchestrator:9090"
register_interval = 30
heartbeat_interval = 10

[metrics]
enable = true
bind_address = "0.0.0.0:9191"

[logging]
level = "info"
format = "json"
```

**Development Configuration** (`config/development.toml`):
```toml
[server]
mode = "tcp"
bind_address = "127.0.0.1:8443"
backend_address = "127.0.0.1:8080"

[logging]
level = "debug"
format = "pretty"

[metrics]
enable = true
bind_address = "127.0.0.1:9191"
```

### Environment Variables

```bash
# Core settings
export MITIGATION_CONFIG="config/production.toml"
export MITIGATION_MODE="l7"
export RUST_LOG="info"

# Security settings
export SYN_COOKIE_SECRET="$(openssl rand -hex 32)"
export TLS_CERT_PATH="/etc/ssl/certs/secbeat.pem"
export TLS_KEY_PATH="/etc/ssl/private/secbeat.key"

# Performance tuning
export WORKER_THREADS="8"
export MAX_CONNECTIONS="10000"
export BUFFER_SIZE="8192"
```

### Dynamic Configuration

**Runtime Configuration Updates**:
```bash
# Reload configuration without restart
curl -X POST http://localhost:9191/api/v1/reload

# Update WAF rules
curl -X POST http://localhost:9191/api/v1/waf/rules \
  -H "Content-Type: application/json" \
  -d @new_rules.json
```

**Orchestrator-Managed Configuration**:
```bash
# Deploy configuration to all nodes
curl -X POST http://orchestrator:9090/api/v1/config/deploy \
  -H "Content-Type: application/json" \
  -d '{"config": "production", "nodes": ["all"]}'
```

## Monitoring and Observability

### Metrics Collection

**Prometheus Metrics** (Port 9191):
```
# Performance metrics
secbeat_requests_total{method="GET",status="200"}
secbeat_response_time_seconds{quantile="0.5"}
secbeat_connections_active
secbeat_bandwidth_bytes_total{direction="in"}

# Security metrics
secbeat_attacks_blocked_total{type="syn_flood"}
secbeat_waf_rules_triggered_total{rule="sql_injection"}
secbeat_blocked_ips_total
secbeat_threat_score

# System metrics
secbeat_cpu_usage_percent
secbeat_memory_usage_bytes
secbeat_uptime_seconds
```

**Custom Metrics**:
```rust
// In application code
metrics::counter!("secbeat_custom_events_total", 1, "type" => "login");
metrics::histogram!("secbeat_processing_time", processing_time);
metrics::gauge!("secbeat_queue_size", queue.len() as f64);
```

### Logging

**Structured Logging** (JSON format):
```json
{
  "timestamp": "2025-01-15T10:30:45Z",
  "level": "INFO",
  "component": "mitigation-node",
  "event": "connection_established",
  "client_ip": "192.168.1.100",
  "backend": "10.0.1.50:8080",
  "connection_id": "conn_12345"
}
```

**Security Event Logging**:
```json
{
  "timestamp": "2025-01-15T10:31:02Z",
  "level": "WARN",
  "component": "waf",
  "event": "attack_detected",
  "attack_type": "sql_injection",
  "client_ip": "192.168.1.200",
  "user_agent": "SQLMap/1.0",
  "blocked": true,
  "rule_id": "rule_001"
}
```

### Dashboards

**Grafana Dashboard Configuration**:
```json
{
  "dashboard": {
    "title": "SecBeat Platform Overview",
    "panels": [
      {
        "title": "Requests per Second",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(secbeat_requests_total[1m])",
            "legendFormat": "RPS"
          }
        ]
      },
      {
        "title": "Security Events",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(secbeat_attacks_blocked_total)",
            "legendFormat": "Blocked Attacks"
          }
        ]
      }
    ]
  }
}
```

### Alerting

**Alert Rules** (Prometheus AlertManager):
```yaml
groups:
  - name: secbeat
    rules:
      - alert: HighErrorRate
        expr: rate(secbeat_requests_total{status=~"5.."}[5m]) > 0.1
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          
      - alert: DDoSAttackDetected
        expr: rate(secbeat_attacks_blocked_total[1m]) > 100
        labels:
          severity: critical
        annotations:
          summary: "DDoS attack in progress"
          
      - alert: NodeDown
        expr: up{job="secbeat"} == 0
        labels:
          severity: critical
        annotations:
          summary: "SecBeat node is down"
```

## Security Features

### DDoS Protection

**SYN Flood Mitigation**:
- Kernel-level packet interception
- SYN cookie generation and validation
- Connection state tracking
- Rate limiting per source IP

**Volumetric Attack Protection**:
- Traffic analysis and profiling
- Anomaly detection algorithms
- Dynamic threshold adjustment
- Intelligent traffic shaping

**Implementation**:
```rust
// SYN proxy logic
async fn handle_syn_packet(packet: &[u8]) -> Result<(), Error> {
    let syn_packet = parse_syn_packet(packet)?;
    
    if is_attack_pattern(&syn_packet) {
        generate_syn_cookie(&syn_packet).await?;
        metrics::counter!("secbeat_syn_cookies_sent", 1);
    } else {
        forward_legitimate_connection(&syn_packet).await?;
    }
    
    Ok(())
}
```

### Web Application Firewall

**Rule Engine**:
- OWASP Core Rule Set integration
- Custom rule development
- Real-time rule updates
- Performance-optimized matching

**Attack Detection**:
- SQL injection prevention
- Cross-site scripting (XSS) protection
- Cross-site request forgery (CSRF) mitigation
- Command injection blocking

**WAF Rules Example**:
```json
{
  "rules": [
    {
      "id": "rule_001",
      "name": "SQL Injection Detection",
      "pattern": "(?i)(union|select|insert|delete|update).*?(from|into|set)",
      "action": "block",
      "severity": "high"
    },
    {
      "id": "rule_002", 
      "name": "XSS Prevention",
      "pattern": "(?i)<script[^>]*>.*?</script>",
      "action": "sanitize",
      "severity": "medium"
    }
  ]
}
```

### AI-Powered Security

**Machine Learning Models**:
- Traffic pattern analysis
- Behavioral anomaly detection
- Attack signature recognition
- Predictive threat modeling

**Threat Intelligence**:
- Real-time IOC feeds
- Reputation-based blocking
- Geolocation filtering
- Behavioral scoring

## Performance Characteristics

### Benchmarks

**Single Node Performance**:
- **Throughput**: 50,000+ RPS
- **Latency**: Sub-3ms proxy overhead
- **Connections**: 10,000+ concurrent
- **Memory**: <100MB base usage

**Multi-Node Cluster**:
- **10 Nodes**: 500,000+ RPS
- **100 Nodes**: 5,000,000+ RPS
- **Scaling**: Linear performance increase
- **Efficiency**: 95%+ resource utilization

### Performance Tuning

**System Optimization**:
```bash
# Kernel tuning for high performance
echo 'net.core.rmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.core.wmem_max = 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_rmem = 4096 87380 134217728' >> /etc/sysctl.conf
echo 'net.ipv4.tcp_wmem = 4096 65536 134217728' >> /etc/sysctl.conf
echo 'net.core.netdev_max_backlog = 5000' >> /etc/sysctl.conf
sysctl -p
```

**Application Tuning**:
```toml
# High-performance configuration
[server]
worker_threads = 16  # 2x CPU cores
max_connections = 20000
buffer_size = 16384
keepalive_timeout = 60

[performance]
enable_tcp_nodelay = true
enable_tcp_fastopen = true
backlog_size = 1024
```

## Operational Procedures

### Day-to-Day Operations

**Health Monitoring**:
```bash
# Check system health
make health-check

# View component status
systemctl status secbeat-*

# Monitor resource usage
htop
iotop
```

**Configuration Management**:
```bash
# Update configuration
vim config/production.toml

# Validate configuration
cargo run --release -- --validate-config

# Reload configuration
curl -X POST http://localhost:9191/api/v1/reload
```

**Certificate Management**:
```bash
# Update TLS certificates
cp new-cert.pem certs/cert.pem
cp new-key.pem certs/key.pem
systemctl reload secbeat-mitigation

# Automated renewal with certbot
certbot renew --deploy-hook "systemctl reload secbeat-mitigation"
```

### Incident Response

**Attack Detection and Response**:
```bash
# View active attacks
curl http://localhost:9191/api/v1/events?type=attack

# Block attacking IP
curl -X POST http://localhost:9191/api/v1/block-ip \
  -d '{"ip": "192.168.1.100", "duration": 3600}'

# Emergency traffic blocking
curl -X POST http://localhost:9191/api/v1/emergency-block \
  -d '{"cidr": "192.168.0.0/16"}'
```

**Performance Issues**:
```bash
# Check resource usage
curl http://localhost:9191/metrics | grep -E "(cpu|memory|connections)"

# View connection statistics
ss -tuln | grep :8443

# Analyze traffic patterns
tail -f logs/mitigation.log | grep "high_traffic"
```

**Recovery Procedures**:
```bash
# Restart failed nodes
systemctl restart secbeat-mitigation

# Fallback to safe configuration
cp config/safe.toml config/production.toml
systemctl reload secbeat-mitigation

# Scale up capacity
curl -X POST http://orchestrator:9090/api/v1/scale \
  -d '{"target_nodes": 10}'
```

### Maintenance

**Regular Tasks**:
```bash
# Log rotation
logrotate -f /etc/logrotate.d/secbeat

# Security updates
cargo update
make build
systemctl restart secbeat-*

# Database cleanup
curl -X POST http://orchestrator:9090/api/v1/cleanup
```

**Backup Procedures**:
```bash
# Configuration backup
tar -czf secbeat-config-$(date +%Y%m%d).tar.gz config/

# Database backup
pg_dump secbeat > secbeat-$(date +%Y%m%d).sql

# Log archival
gzip logs/*.log.1
aws s3 cp logs/ s3://backups/secbeat/logs/ --recursive
```

## Troubleshooting Guide

### Common Issues

**Connection Issues**:
```bash
# Check port availability
netstat -tlnp | grep :8443

# Test connectivity
telnet localhost 8443

# Verify backend health
curl http://localhost:8080/health
```

**Performance Issues**:
```bash
# Check CPU usage
top -p $(pgrep mitigation-node)

# Memory analysis
cat /proc/$(pgrep mitigation-node)/status

# Network statistics
iftop -i eth0
```

**Configuration Issues**:
```bash
# Validate configuration syntax
cargo run --release -- --validate-config

# Check file permissions
ls -la config/ certs/

# Verify environment variables
env | grep MITIGATION
```

### Error Resolution

**Build Errors**:
```bash
# Clean build cache
cargo clean

# Update dependencies
cargo update

# Check Rust version
rustc --version
```

**Runtime Errors**:
```bash
# Check logs for errors
tail -f logs/mitigation.log | grep ERROR

# Verify system resources
free -h
df -h

# Check network interfaces
ip addr show
```

**Security Issues**:
```bash
# Check firewall rules
iptables -L -n

# Verify certificate validity
openssl x509 -in certs/cert.pem -text -noout

# Test TLS configuration
openssl s_client -connect localhost:8443
```

This platform guide provides comprehensive information for deploying, configuring, and operating SecBeat in production environments. For additional support, refer to the API documentation, community forums, or file an issue in the GitHub repository.
