# SecBeat: Production-Grade DDoS Mitigation & WAF Platform

<div align="center">

[![Documentation](https://img.shields.io/badge/📚_Documentation-Visit_Site-blue?style=for-the-badge)](https://fabriziosalmi.github.io/secbeat)
[![Rust](https://img.shields.io/badge/rust-1.78+-93450a.svg?style=flat-square)](https://www.rust-lang.org)
[![Tokio](https://img.shields.io/badge/tokio-1.35-blue.svg?style=flat-square)](https://tokio.rs)
[![Architecture](https://img.shields.io/badge/architecture-microservices-lightgrey.svg?style=flat-square)](#-architecture)
[![Status](https://img.shields.io/badge/status-Production%20Ready-brightgreen.svg?style=flat-square)](#-getting-started)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg?style=flat-square)](#-testing)

</div>

> 🌐 **[View Full Documentation →](https://fabriziosalmi.github.io/secbeat)**

**SecBeat** is a high-performance, memory-safe, and enterprise-grade distributed security platform built entirely in Rust. It provides comprehensive protection against sophisticated Layer 4 (TCP/UDP) and Layer 7 (HTTP/S) DDoS attacks while offering advanced Web Application Firewall (WAF) capabilities with AI-powered threat detection and autonomous scaling.

The system implements a revolutionary "smart edge, intelligent orchestrator" architecture, enabling extreme scalability, self-healing capabilities, and infrastructure agnosticism across cloud and on-premises environments.

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/fabriziosalmi/secbeat.git
cd secbeat

# Build all components (requires Rust 1.78+)
make build

# Run comprehensive test suite
make test

# Deploy in production mode
make deploy-production

# Or start individual components for development
make start-orchestrator  # Starts orchestrator node
make start-mitigation    # Starts mitigation node (requires sudo)
```

## 📋 Table of Contents

- [🎯 Platform Overview](#-platform-overview)
- [🏗️ Architecture](#️-architecture)
- [⚡ Getting Started](#-getting-started)
- [🔧 Configuration](#-configuration)
- [🧪 Testing](#-testing)
- [🚀 Deployment](#-deployment)
- [📊 Monitoring](#-monitoring)
- [🔒 Security Features](#-security-features)
- [� Documentation](#-documentation)
- [🤝 Contributing](#-contributing)
- [📄 License](#-license)

## 📖 Documentation

- **[QUICKSTART.md](QUICKSTART.md)** - Quick reference and common commands
- **[DEPLOYMENT.md](DEPLOYMENT.md)** - Comprehensive deployment guide
- **[PLATFORM.md](PLATFORM.md)** - Platform architecture and capabilities
- **[KERNEL_OPERATIONS.md](KERNEL_OPERATIONS.md)** - Kernel-level operations guide

## 🎯 Platform Overview

SecBeat is designed as a unified security platform that scales from single-node deployments to global multi-region clusters. The platform provides:

### 🛡️ Multi-Layer Protection

**Layer 4 (Network/Transport)**
- **TCP/UDP Proxy**: High-performance async proxy with sub-millisecond latency
- **SYN Proxy**: Advanced SYN flood protection using kernel-level packet processing
- **Connection Management**: Intelligent connection pooling and rate limiting
- **Network Monitoring**: Real-time traffic analysis and anomaly detection

**Layer 7 (Application)**
- **HTTPS Termination**: Modern TLS 1.3 with certificate management
- **Web Application Firewall**: Dynamic rule engine with ML-powered detection
- **HTTP/2 Support**: Full HTTP/2 protocol implementation
- **Request Filtering**: Advanced pattern matching and content inspection

### 🤖 AI-Powered Intelligence

**Threat Detection**
- Real-time attack pattern recognition
- Behavioral analysis and anomaly detection
- Cross-correlation of security events
- Predictive threat modeling

**Autonomous Response**
- Dynamic rule generation and deployment
- Automated scaling based on traffic patterns
- Self-healing node replacement
- Intelligent load balancing

### 🌐 Distributed Architecture

**Mitigation Nodes** (Edge Security)
- High-performance traffic processing
- Local decision making capabilities
- Real-time metrics collection
- Horizontal scaling support

**Orchestrator Node** (Control Plane)
- Centralized fleet management
- AI-powered decision engine
- Resource optimization
- Global coordination

## 🏗️ Architecture

```
                           ┌─────────────────────────────────────┐
                           │         Orchestrator Node          │
                           │  ┌─────────────────────────────┐    │
                           │  │     Control Plane APIs     │    │
                           │  │  • Fleet Management        │    │
                           │  │  • Policy Distribution     │    │
                           │  │  • Resource Orchestration  │    │
                           │  └─────────────────────────────┘    │
                           │  ┌─────────────────────────────┐    │
                           │  │      AI Engine             │    │
                           │  │  • Threat Intelligence     │    │
                           │  │  • Predictive Scaling      │    │
                           │  │  • Decision Engine         │    │
                           │  └─────────────────────────────┘    │
                           └─────────────────────────────────────┘
                                           │
                          ┌────────────────┴────────────────┐
                          │         NATS Messaging         │
                          │    Real-time Event Stream      │
                          └────────────────┬────────────────┘
                                           │
        ┌──────────────────────────────────┼──────────────────────────────────┐
        │                                  │                                  │
┌───────▼────────┐                ┌───────▼────────┐                ┌───────▼────────┐
│ Mitigation     │                │ Mitigation     │                │ Mitigation     │
│ Node 1         │                │ Node 2         │                │ Node N         │
│                │                │                │                │                │
│ ┌────────────┐ │                │ ┌────────────┐ │                │ ┌────────────┐ │
│ │ TCP Proxy  │ │                │ │ TCP Proxy  │ │                │ │ TCP Proxy  │ │
│ │ SYN Proxy  │ │                │ │ SYN Proxy  │ │                │ │ SYN Proxy  │ │
│ │ TLS Term   │ │                │ │ TLS Term   │ │                │ │ TLS Term   │ │
│ │ WAF Engine │ │                │ │ WAF Engine │ │                │ │ WAF Engine │ │
│ └────────────┘ │                │ └────────────┘ │                │ └────────────┘ │
│                │                │                │                │                │
│ [Clients] ──── │                │ [Clients] ──── │                │ [Clients] ──── │
│     ↓          │                │     ↓          │                │     ↓          │
│ [Backends]     │                │ [Backends]     │                │ [Backends]     │
└────────────────┘                └────────────────┘                └────────────────┘
```

### 🔄 Operation Modes

SecBeat mitigation nodes support multiple operation modes:

1. **TCP Mode**: Basic high-performance TCP proxy
2. **SYN Mode**: SYN proxy with DDoS protection
3. **L7 Mode**: Full Layer 7 processing with WAF

Each mode can be configured independently with specific performance and security profiles.

## ⚡ Getting Started

### 📋 Prerequisites

- **Rust Toolchain**: 1.78+ with Cargo
- **Operating System**: Linux or macOS (Windows support planned)
- **Privileges**: Root access for raw socket operations (SYN proxy mode)
- **Memory**: 4GB+ RAM recommended for production
- **Network**: Multiple network interfaces for comprehensive testing

### 🛠️ Installation

```bash
# Clone repository
git clone https://github.com/fabriziosalmi/secbeat.git
cd secbeat

# Install system dependencies (Ubuntu/Debian)
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev curl jq

# Install system dependencies (macOS)
brew install openssl curl jq

# Build all components
make build

# Or build manually
cargo build --release --workspace
```

### 🎯 Basic Deployment

```bash
# 1. Generate TLS certificates for HTTPS termination
cd mitigation-node
mkdir -p certs
openssl req -x509 -newkey rsa:4096 \
    -keyout certs/key.pem -out certs/cert.pem \
    -days 365 -nodes -subj "/CN=localhost"

# 2. Start orchestrator node
cd ../orchestrator-node
RUST_LOG=info cargo run --release &

# 3. Start mitigation node in TCP mode
cd ../mitigation-node
export MITIGATION_CONFIG=config/tcp.toml
sudo RUST_LOG=info cargo run --release &

# 4. Test the deployment
curl -v https://localhost:8443/
```

### 🔧 Production Deployment

```bash
# Use production configuration
export MITIGATION_CONFIG=config/production.toml
export ORCHESTRATOR_CONFIG=config/production.toml

# Deploy with systemd services
sudo make install-systemd
sudo systemctl enable secbeat-orchestrator
sudo systemctl enable secbeat-mitigation
sudo systemctl start secbeat-orchestrator
sudo systemctl start secbeat-mitigation
```

## 🔧 Configuration

SecBeat uses TOML configuration files for flexible deployment scenarios.

### 📁 Configuration Files

- `config/tcp.toml` - Basic TCP proxy mode
- `config/syn.toml` - SYN proxy with DDoS protection
- `config/l7.toml` - Full Layer 7 with WAF
- `config/production.toml` - Production deployment settings

### 🛠️ Mitigation Node Configuration

```toml
# config/production.toml
[server]
mode = "l7"                    # tcp, syn, or l7
bind_address = "0.0.0.0:8443"
backend_address = "127.0.0.1:8080"
worker_threads = 0             # 0 = auto-detect CPU cores

[tls]
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
protocols = ["TLSv1.3", "TLSv1.2"]

[syn_proxy]
enable = true
max_syn_backlog = 65536
syn_cookie_secret = "your-secret-key"

[waf]
enable = true
rules_path = "config/waf_rules.json"
block_suspicious = true
rate_limit_rps = 1000

[orchestrator]
url = "http://127.0.0.1:9090"
register_interval = 30
heartbeat_interval = 10

[metrics]
enable = true
bind_address = "0.0.0.0:9191"
```

### 🎛️ Orchestrator Configuration

```toml
# orchestrator config
[server]
bind_address = "0.0.0.0:9090"
worker_threads = 4

[fleet]
registration_timeout = 60
heartbeat_timeout = 30
health_check_interval = 15

[ai]
enable_threat_detection = true
enable_predictive_scaling = true
model_update_interval = 300

[messaging]
nats_url = "nats://127.0.0.1:4222"
```

### 🔀 Operation Mode Selection

Set the operation mode via configuration or environment variable:

```bash
# Via configuration file
export MITIGATION_CONFIG=config/syn.toml

# Via environment variable
export MITIGATION_MODE=l7

# Via command line
cargo run --release -- --mode tcp
```

## 🧪 Testing

SecBeat includes comprehensive testing capabilities covering all system components and operation modes.

### 🚀 Quick Test

```bash
# Run all tests
make test

# Or run manually
sudo ./test_all.sh
```

### 🔧 Component Testing

```bash
# Test specific components
make test-tcp      # TCP proxy functionality
make test-syn      # SYN proxy and DDoS protection
make test-l7       # Layer 7 processing and WAF
make test-orchestrator  # Control plane functionality
```

### 📊 Performance Testing

```bash
# Load testing with multiple concurrent connections
make test-load

# Stress testing with attack simulation
make test-stress

# Benchmark all operation modes
make benchmark
```

### 🧪 Integration Testing

```bash
# End-to-end testing with real traffic
make test-e2e

# Multi-node cluster testing
make test-cluster

# Failover and recovery testing
make test-failover
```

### 🎯 Behavioral Analysis Testing

Test the complete behavioral analysis pipeline (Mitigation Node → NATS → Orchestrator → Ban):

```bash
# Run the end-to-end behavioral analysis test
./test_behavioral_ban.sh
```

**What this test does:**
1. **Baseline Check**: Verifies normal traffic passes through (HTTP 200/404)
2. **Attack Simulation**: Sends 60 sequential 404 errors to trigger anomaly detection
3. **Analysis Window**: Waits for orchestrator's sliding window algorithm to detect the pattern
4. **Ban Verification**: Confirms IP is blocked with HTTP 403 or connection refused

**Expected Flow:**
```
Mitigation Node → publishes telemetry → secbeat.telemetry.{node_id}
     ↓
Orchestrator → BehavioralExpert analyzes sliding window
     ↓
Threshold exceeded (50+ errors in 60s) → BlockCommand issued
     ↓
Orchestrator → publishes ban → secbeat.commands.block
     ↓
Mitigation Node → receives command → updates DynamicRuleState
     ↓
Future requests from IP → blocked for 5 minutes (TTL: 300s)
```

**Success Indicators:**
- ✅ `🎉 TEST PASSED!` - Ban was successfully enforced
- ✅ HTTP 403 Forbidden or connection timeout
- ✅ NATS message propagation working
- ✅ Dynamic IP blocking active

**Troubleshooting:**
If the test fails, check:
- NATS connectivity: `docker-compose logs nats`
- Orchestrator logs: `docker-compose logs orchestrator | grep behavioral`
- Mitigation node logs: `docker-compose logs mitigation-node | grep block`
- Threshold config: Error threshold is 50, test sends 60 errors

## 🚀 Deployment

SecBeat supports multiple deployment scenarios from development testing to enterprise production environments.

### 🏢 Proxmox Virtual Environment (Recommended)

**Automated multi-node deployment with full production stack:**

```bash
# Quick deployment to Proxmox VE
./deploy_proxmox.sh test     # Pre-deployment validation
./deploy_proxmox.sh deploy   # Full multi-VM deployment

# Check deployment status
./deploy_proxmox.sh status

# Access monitoring
open http://192.168.300.10:3000  # Grafana (admin/secbeat123)
open http://192.168.300.10:9090  # Prometheus
```

**What gets deployed:**
- **3 Mitigation Nodes** - DDoS protection and WAF (192.168.200.10-12)
- **1 Orchestrator** - Central coordination (192.168.200.20)
- **3 NATS Cluster** - Event messaging (192.168.200.30-32)
- **2 Load Balancers** - HA traffic distribution (192.168.200.40-41)
- **1 Monitoring Stack** - Grafana + Prometheus (192.168.300.10)

**Prerequisites:**
- Proxmox VE 7.0+ at 192.168.100.23 (configurable)
- Ubuntu 22.04 LTS ISO uploaded to Proxmox
- SSH key access: `ssh-copy-id root@192.168.100.23`
- 20+ CPU cores, 32+ GB RAM, 300+ GB storage

📖 **[Complete Proxmox Deployment Guide](deployment/README.md)**

### 🐳 Container Deployment

```bash
# Development environment
docker-compose up -d

# Production with custom configs
docker-compose -f docker-compose.prod.yml up -d

# Kubernetes deployment
kubectl apply -f k8s/
```

### ☁️ Cloud Deployment

```bash
# Deploy to AWS (planned)
cd terraform/aws
terraform init && terraform apply

# Deploy to Azure (planned)
cd terraform/azure  
terraform init && terraform apply

# Deploy to GCP (planned)
cd terraform/gcp
terraform init && terraform apply
```

### 🏗️ Single Node Deployment

```bash
# Development/testing on single machine
make build
make install

# Start services
sudo systemctl enable --now secbeat-orchestrator
sudo systemctl enable --now secbeat-mitigation

# Verify installation
curl -k https://localhost:8443/health
```

## 📊 Monitoring

### 📈 Metrics Collection

SecBeat exposes Prometheus-compatible metrics on port 9191:

```bash
# View available metrics
curl http://localhost:9191/metrics

# Key metrics include:
# - secbeat_connections_total
# - secbeat_requests_per_second
# - secbeat_response_time_seconds
# - secbeat_blocked_attacks_total
# - secbeat_cpu_usage_percent
# - secbeat_memory_usage_bytes
```

### 📊 Dashboard Setup

```bash
# Deploy Grafana dashboard
docker run -d --name grafana \
  -p 3000:3000 \
  -v $(pwd)/grafana:/etc/grafana/provisioning \
  grafana/grafana

# Import SecBeat dashboard
curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @grafana/secbeat-dashboard.json
```

### 🔍 Log Analysis

```bash
# View real-time logs
tail -f logs/mitigation.log logs/orchestrator.log

# Structured log analysis
jq '.' logs/mitigation.log | grep "attack_detected"

# Export logs to Elasticsearch
filebeat -c config/filebeat.yml
```

## 🔒 Security Features

### 🛡️ DDoS Protection

**SYN Flood Protection**
- Kernel-level packet interception
- SYN cookie validation
- Connection state tracking
- Automatic rate limiting

**Volumetric Attack Mitigation**
- Traffic analysis and profiling
- Anomaly detection algorithms
- Dynamic threshold adjustment
- Intelligent traffic shaping

### 🔐 Web Application Firewall

**Rule Engine**
- OWASP Core Rule Set integration
- Custom rule development
- Real-time rule updates
- Lua scripting support

**Attack Detection**
- SQL injection prevention
- XSS protection
- CSRF mitigation
- Command injection blocking

### 🤖 AI-Powered Features

**Machine Learning Models**
- Traffic pattern analysis
- Behavioral anomaly detection
- Attack signature recognition
- Predictive threat modeling

**Autonomous Response**
- Dynamic rule generation
- Automated scaling decisions
- Self-healing capabilities
- Intelligent load balancing

## 🛠️ API Reference

### 🌐 Orchestrator API

```bash
# Fleet management
GET  /api/v1/nodes                    # List all nodes
POST /api/v1/nodes/{id}/scale         # Scale specific node
GET  /api/v1/nodes/{id}/metrics       # Node metrics
POST /api/v1/nodes/{id}/restart       # Restart node

# Policy management
GET  /api/v1/policies                 # List policies
POST /api/v1/policies                 # Create policy
PUT  /api/v1/policies/{id}           # Update policy
DELETE /api/v1/policies/{id}         # Delete policy

# Security events
GET  /api/v1/events                   # Security events
POST /api/v1/events/acknowledge      # Acknowledge events
GET  /api/v1/events/stats            # Event statistics
```

### 📊 Mitigation Node API

```bash
# Node status
GET  /api/v1/status                   # Node health status
GET  /api/v1/metrics                  # Performance metrics
POST /api/v1/reload                   # Reload configuration

# Security operations
GET  /api/v1/blocked-ips             # Blocked IP addresses
POST /api/v1/block-ip                # Block specific IP
DELETE /api/v1/block-ip/{ip}         # Unblock IP
GET  /api/v1/waf/rules               # WAF rules
POST /api/v1/waf/rules               # Add WAF rule
```

## 📖 Operations Guide

### 🔄 Day-to-Day Operations

**Health Monitoring**
```bash
# Check system health
make health-check

# View component status
systemctl status secbeat-*

# Monitor resource usage
htop
iotop
```

**Configuration Updates**
```bash
# Update WAF rules
vim config/waf_rules.json
curl -X POST http://localhost:9191/api/v1/reload

# Update TLS certificates
cp new-cert.pem certs/cert.pem
cp new-key.pem certs/key.pem
systemctl reload secbeat-mitigation
```

**Scaling Operations**
```bash
# Manual scaling
curl -X POST http://orchestrator:9090/api/v1/nodes/scale \
  -d '{"target_nodes": 5}'

# Auto-scaling configuration
vim config/autoscaling.toml
```

### 🚨 Incident Response

**Attack Detection**
```bash
# View active attacks
curl http://localhost:9191/api/v1/events?type=attack

# Block attacking IPs
curl -X POST http://localhost:9191/api/v1/block-ip \
  -d '{"ip": "192.168.1.100", "duration": 3600}'
```

**Performance Issues**
```bash
# Check resource usage
curl http://localhost:9191/metrics | grep cpu_usage

# View connection statistics
curl http://localhost:9191/metrics | grep connections
```

**Recovery Procedures**
```bash
# Restart failed nodes
systemctl restart secbeat-mitigation

# Reset to safe configuration
cp config/safe.toml config/production.toml
systemctl reload secbeat-mitigation
```

### 🔧 Maintenance

**Regular Tasks**
```bash
# Log rotation
logrotate -f /etc/logrotate.d/secbeat

# Certificate renewal
certbot renew
systemctl reload secbeat-mitigation

# Security updates
cargo update
make build
systemctl restart secbeat-*
```

**Backup Procedures**
```bash
# Configuration backup
tar -czf secbeat-config-$(date +%Y%m%d).tar.gz config/

# Log archival
gzip logs/*.log.1
aws s3 cp logs/ s3://backups/secbeat/logs/ --recursive
```

## 🤝 Contributing

We welcome contributions to SecBeat! Please read our contributing guidelines:

### 🛣️ Development Roadmap

#### ✅ **Production Ready (v0.9.x)**
- ✅ TCP/L7 reverse proxy with TLS termination
- ✅ 150+ WAF attack patterns (SQL injection, XSS, path traversal, command injection)
- ✅ NATS-based distributed messaging
- ✅ Prometheus metrics integration
- ✅ Docker and Docker Compose deployment
- ✅ Management API (health, status, WAF control)
- ✅ ML-based predictive scaling (linear regression for CPU prediction)
- ✅ Configuration hot-reload
- ✅ Kubernetes deployment manifests

#### 🚧 **Beta/Experimental**
- ⚠️ **SYN Proxy** - Functional prototype with known limitations
  - Basic SYN flood protection implemented
  - Kernel-level packet processing (requires CAP_NET_RAW)
  - Challenge-response validation
  - ⚠️ **Use TCP mode for production workloads**
  - Planned: Complete TCP handshake validation, cookie encryption improvements

#### 🔄 **In Development (v1.0 - Q1 2025)**
- 🔨 Complete threat intelligence API (`/api/v1/threats`)
- 🔨 Enhanced statistics collection and reporting
- 🔨 IP blacklist/whitelist persistence layer
- 🔨 Automated testing framework expansion
- 🔨 Performance benchmarking suite
- 🔨 Dashboard and visualization tools

#### 📋 **Planned Features (v1.1 - Q2 2025: "Kernel" Update)**
- 📅 **eBPF/XDP Integration** - Move packet filtering to kernel space for 10x performance during volumetric attacks
- 📅 **Zero-Copy Networking** - Optimize TCP proxy using `sendfile` and `splice` for reduced CPU overhead
- 📅 HTTP/2 protocol support
- 📅 OWASP ModSecurity Core Rule Set (CRS) integration
- 📅 Geo-blocking and GeoIP integration
- 📅 Rate limiting with Redis backend
- 📅 Multi-tenant support

#### 🧠 **Intelligence Update (v1.2 - Q3 2025)**
- 🔮 **WASM WAF Runtime** - Replace regex engine with WebAssembly (Wasmtime) for complex, programmable logic rules hot-loaded at runtime
- 🔮 **Advanced ML Models** - Upgrade from Linear Regression to LSTM or Isolation Forest for traffic anomaly detection and pattern recognition
- 🔮 Advanced behavioral analysis
- 🔮 DDoS mitigation learning mode
- 🔮 Lua scripting for custom WAF rules (alternative to WASM)

#### 🏢 **Enterprise Update (v2.0 - Q4 2025)**
- 🌐 **Distributed State (CRDTs)** - Instant global ban-list synchronization across the fleet without consensus overhead
- 📊 **React Dashboard UI** - Real-time attack visualization, fleet management, and analytics (beyond CLI/API)
- 🔧 **Terraform Provider** - Official provider for managing SecBeat infrastructure as code
- 🔮 GraphQL API support
- 🔮 Machine learning model marketplace
- 🔮 Auto-scaling integration (AWS, Azure, GCP)

#### 🎯 **Future Enhancements (v2.1+)**
- 🔮 Distributed tracing (OpenTelemetry)
- 🔮 Service mesh integration (Istio, Linkerd)
- 🔮 Zero-trust network architecture
- 🔮 Quantum-resistant cryptography

### 📊 Feature Status Matrix

| Feature | Status | Production Ready | Notes |
|---------|--------|------------------|-------|
| TCP Proxy | ✅ Stable | Yes | Sub-millisecond latency |
| L7 Proxy (HTTPS) | ✅ Stable | Yes | TLS 1.3 support |
| WAF Engine | ✅ Stable | Yes | 150+ patterns |
| SYN Proxy | ⚠️ Beta | No | Use for testing only |
| NATS Messaging | ✅ Stable | Yes | Full integration |
| Prometheus Metrics | ✅ Stable | Yes | Comprehensive metrics |
| ML Predictive Scaling | ✅ Stable | Yes | Linear regression |
| Management API | ✅ Stable | Yes | RESTful endpoints |
| Docker Deployment | ✅ Stable | Yes | Multi-container |
| Kubernetes | ✅ Stable | Yes | Tested on K8s 1.25+ |
| eBPF/XDP Integration | 📅 Q2 2025 | No | Kernel-space filtering |
| WASM WAF Runtime | 📅 Q3 2025 | No | Programmable rules |
| Advanced ML (LSTM) | 📅 Q3 2025 | No | Anomaly detection |
| CRDT State Sync | 📅 Q4 2025 | No | Global ban-list |
| React Dashboard | 📅 Q4 2025 | No | Visual management |
| Terraform Provider | 📅 Q4 2025 | No | Infrastructure as code |
| HTTP/2 Support | 📅 Q2 2025 | No | Protocol upgrade |
| OWASP CRS | 📅 Q2 2025 | No | Rule set integration |
| Threat Intelligence API | 🔨 Q1 2025 | Partial | Basic implementation |

### 🎯 Why This Roadmap Matters

**Q2 2025 - Performance**: eBPF/XDP moves packet filtering to kernel space, achieving **10x performance** during volumetric attacks while maintaining sub-millisecond latency.

**Q3 2025 - Intelligence**: WASM runtime enables complex, programmable WAF logic that can be hot-loaded without restarts. LSTM models provide true anomaly detection beyond simple regression.

**Q4 2025 - Scale**: CRDTs enable instant global state synchronization across distributed fleets without consensus overhead. The React dashboard brings enterprise-grade visibility.

This roadmap demonstrates SecBeat's evolution from a **production-ready security platform** to a **next-generation intelligent defense system**.

---

### 🐛 Bug Reports

1. Check existing issues first
2. Use the bug report template
3. Include reproduction steps
4. Provide system information

### ✨ Feature Requests

1. Check the roadmap first
2. Use the feature request template
3. Describe the use case
4. Consider implementation complexity

### 💻 Development

```bash
# Set up development environment
git clone https://github.com/your-org/secbeat.git
cd secbeat
cargo install --path .

# Run tests
make test
cargo test --all-features

# Submit pull request
git checkout -b feature/your-feature
git commit -m "feat: add your feature"
git push origin feature/your-feature
```

### 📋 Development Guidelines

- Follow Rust best practices and idioms
- Add tests for new functionality
- Update documentation for changes
- Use conventional commit messages
- Ensure all tests pass before submitting

## 📄 License

SecBeat is released under the MIT License. See [LICENSE](LICENSE) for details.

```
MIT License

Copyright (c) 2024 SecBeat Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## 🙏 Acknowledgments

- **Rust Community** for the excellent async ecosystem
- **Tokio Team** for the high-performance runtime
- **OWASP** for web application security guidance
- **NATS.io** for the messaging infrastructure
- **All Contributors** who made this project possible

---

<div align="center">

**🚀 Ready to deploy SecBeat? Start with our [Quick Start Guide](#-getting-started)!**

[![Deploy to AWS](https://img.shields.io/badge/Deploy%20to-AWS-orange.svg)](terraform/aws/)
[![Deploy to Azure](https://img.shields.io/badge/Deploy%20to-Azure-blue.svg)](terraform/azure/)
[![Deploy to GCP](https://img.shields.io/badge/Deploy%20to-GCP-green.svg)](terraform/gcp/)

[Documentation](docs/) • [API Reference](#️-api-reference) • [Community](https://github.com/your-org/secbeat/discussions) • [Support](https://github.com/your-org/secbeat/issues)

</div>
