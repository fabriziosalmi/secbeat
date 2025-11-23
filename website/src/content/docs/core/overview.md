---
title: Core Architecture Overview
description: Understanding SecBeat's architecture and capabilities
slug: core/overview
---

## Platform Architecture

SecBeat implements a distributed "smart edge, intelligent orchestrator" architecture that provides both high-performance traffic processing and centralized intelligence.

### System Components

#### Mitigation Nodes
High-performance traffic processing engines at the edge:
- Multiple operation modes
- Real-time threat detection
- Local decision-making
- Horizontal scaling

#### Orchestrator Node
Centralized control plane and intelligence:
- Fleet management
- AI-powered decisions
- Resource optimization
- Policy distribution

#### Communication Layer
NATS-based real-time messaging:
- Real-time messaging
- RESTful APIs
- Webhook integration
- Metrics streaming

### Data Flow

```
Internet Traffic → Mitigation Nodes → Backend Services
                        ↕
                Orchestrator Node (Control & Intelligence)
```

## Core Capabilities

### Multi-Layer Protection

#### Layer 4 (Network/Transport)
- TCP/UDP proxy with sub-millisecond latency
- SYN flood protection using kernel-level packet processing
- Connection rate limiting and state tracking
- Network-level DDoS mitigation

#### Layer 7 (Application)
- HTTPS termination with modern TLS support
- Web Application Firewall with 100+ attack patterns
- Request filtering and content inspection
- Pattern-based threat detection

### ML-Powered Resource Management

#### Predictive Scaling
- Linear regression CPU prediction
- Resource trend analysis
- Proactive capacity planning
- Historical data modeling

#### Autonomous Response
- Pattern-based threat detection
- Automated scaling decisions
- Self-healing nodes
- Intelligent load balancing

## Operation Modes

SecBeat mitigation nodes support three primary operation modes, each optimized for specific security and performance requirements.

### TCP Mode
**Use Case:** High-performance reverse proxy

- Ultra-low latency (<0.5ms)
- Millions of connections/sec
- Minimal CPU overhead
- No root privileges required

### SYN Mode (Beta)
**Use Case:** DDoS mitigation layer

:::caution
Functional prototype with known limitations. Use TCP mode for production workloads.
:::

- SYN flood protection
- Kernel-level packet filtering
- Challenge-response validation
- Requires CAP_NET_RAW

### L7 Mode
**Use Case:** Complete security suite

- Full WAF capabilities
- TLS termination
- Content inspection
- Advanced threat detection

## Platform Features

### High Availability
- Distributed architecture with no single points of failure
- Automatic failover and recovery
- Graceful degradation under load
- Health monitoring and self-healing

### Scalability
- Horizontal scaling of mitigation nodes
- Predictive scaling based on ML models
- Dynamic resource allocation
- Cloud-agnostic deployment

### Observability

SecBeat exposes comprehensive metrics via Prometheus:

```
secbeat_packets_processed_total
secbeat_attacks_blocked_total
secbeat_latency_seconds
secbeat_cpu_usage_percent
secbeat_memory_usage_bytes
```

## Security Features

### DDoS Protection

#### Volumetric Attacks
- UDP floods
- ICMP floods
- DNS amplification
- NTP amplification

#### Protocol Attacks
- SYN floods
- ACK floods
- Fragment attacks
- Slowloris

#### Application Attacks
- HTTP floods
- Slow POST
- Cache busting
- API abuse

### WAF Capabilities
- 100+ regex-based attack patterns
- SQL injection prevention
- XSS filtering
- Command injection blocking
- Path traversal detection
- Pattern-based detection engine

## Performance Characteristics

### Key Metrics

| Metric | Value |
|--------|-------|
| Packets/Second | 2.5M+ |
| Average Latency | 0.3ms |
| Concurrent Connections | 100K+ |
| Uptime SLA | 99.99% |

### Benchmarks

```
TCP Mode:    2.5M packets/sec, 0.2ms latency
SYN Mode:    1.8M packets/sec, 0.4ms latency
L7 Mode:     500K requests/sec, 1.2ms latency

Memory:      256MB base + 10KB per connection
CPU:         12% at 100K connections
Threads:     Auto-scaled based on cores
```

## Next Steps

- [SYN Flood Mitigation](/core/syn-flood/) - Deep dive into SYN protection
- [Observability](/core/observability/) - Monitoring and metrics
- [XDP Programs](/kernel/xdp/) - Kernel-level packet processing
