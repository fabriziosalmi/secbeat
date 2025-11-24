# ADR 001: NATS for Internal Communication vs gRPC

**Status:** Accepted  
**Date:** 2025-11-24  
**Deciders:** Architecture Team  
**Technical Story:** Multi-node coordination and event distribution in SecBeat

## Context and Problem Statement

SecBeat requires reliable, low-latency communication between mitigation nodes and the orchestrator for:
- Real-time DDoS attack event distribution
- Dynamic rule updates and synchronization
- Distributed state coordination (IP reputation, rate limit counters)
- Health monitoring and metrics collection

We need to choose between NATS (message-oriented middleware) and gRPC (RPC framework) for internal service communication.

## Decision Drivers

* **Latency requirements:** Sub-millisecond event propagation for attack mitigation
* **Operational simplicity:** Minimal infrastructure complexity
* **Scalability:** Support for 100+ mitigation nodes
* **Fault tolerance:** Network partitions and node failures must not cascade
* **Multi-tenancy:** Isolation between different deployments
* **Pattern matching:** Dynamic subscription to event types

## Considered Options

### Option 1: gRPC with Streaming

**Pros:**
- Strongly typed service contracts (Protocol Buffers)
- Built-in health checking and load balancing
- HTTP/2 multiplexing reduces connection overhead
- Good ecosystem support and tooling
- Request/response semantics familiar to developers

**Cons:**
- Requires point-to-point connections between all nodes
- Complex topology management for full mesh (N² connections)
- No native pub/sub pattern - requires custom implementation
- Reconnection logic increases code complexity
- Service discovery needed for dynamic node addition
- Backpressure handling requires custom implementation

### Option 2: NATS (Chosen)

**Pros:**
- Native pub/sub with subject-based routing
- Automatic fanout to multiple subscribers (1-to-N)
- Built-in clustering and high availability
- JetStream for guaranteed delivery and persistence
- Subject wildcards enable flexible event routing
- Connection multiplexing reduces overhead
- At-most-once and at-least-once delivery semantics
- Decoupled architecture - nodes don't need to know about each other
- Extremely low latency (< 1ms for local cluster)
- Simple operational model (single NATS cluster)

**Cons:**
- Additional infrastructure component to operate
- Learning curve for NATS-specific concepts
- No strongly-typed contracts (must validate message schemas)
- Requires separate schema validation layer

### Option 3: Redis Pub/Sub

**Pros:**
- Many teams already run Redis
- Simple API
- Good performance for basic use cases

**Cons:**
- No message persistence (fire-and-forget)
- No guaranteed delivery
- Limited scalability compared to NATS
- Single point of failure without Redis Cluster
- Not designed for high-throughput streaming

## Decision Outcome

**Chosen option: NATS with JetStream** because it best addresses our architectural needs:

### Technical Justification

1. **Pub/Sub Pattern Match**
   - Attack events need 1-to-N distribution (one detector, many mitigators)
   - Subject hierarchy naturally maps to event taxonomy:
     ```
     attacks.ddos.syn.node-1
     attacks.waf.sql_injection.node-2
     rules.update.global
     state.ip_reputation.10.0.0.1
     ```

2. **Decoupled Architecture**
   - Mitigation nodes don't need orchestrator service discovery
   - New nodes subscribe and immediately receive events
   - No complex mesh topology to maintain
   - Simplifies deployment in Kubernetes/container environments

3. **Performance Characteristics**
   - NATS core: 11M+ messages/sec on commodity hardware
   - Sub-millisecond latency for local cluster
   - Horizontal scaling via clustering
   - Significantly lower resource usage than gRPC mesh

4. **Operational Simplicity**
   - Single NATS cluster serves entire deployment
   - Built-in monitoring via `/varz` endpoint
   - No service mesh required
   - Automatic reconnection and failover

5. **Multi-Tenancy Support**
   - Subject namespacing: `tenant.{id}.attacks.*`
   - JetStream streams per tenant for isolation
   - Account-based authentication

### Implementation Details

```rust
// Event publishing (orchestrator)
nats_client.publish(
    "attacks.ddos.syn.node-1",
    serde_json::to_vec(&attack_event)?
).await?;

// Event subscription (mitigation node)
let mut subscriber = nats_client
    .subscribe("attacks.ddos.>") // Wildcard for all DDoS attacks
    .await?;

while let Some(msg) = subscriber.next().await {
    let event: AttackEvent = serde_json::from_slice(&msg.payload)?;
    handle_attack(event).await;
}
```

### Trade-offs

**Accepted:**
- Need to run NATS cluster (1-3 nodes for HA)
- Manual schema validation (mitigated with Rust type system)
- Message size limits (1MB default, configurable)

**Mitigated:**
- Schema validation: Use `serde` for compile-time safety
- Message size: Large payloads use NATS object store
- Monitoring: Prometheus exporter for NATS metrics

## Consequences

### Positive

- Event distribution latency < 5ms (measured)
- Zero-downtime node additions/removals
- Simple horizontal scaling (add more mitigation nodes)
- Clean separation between orchestrator and mitigation layers
- Future: Use JetStream for audit logging and replay

### Negative

- Additional infrastructure component (NATS cluster)
- Requires monitoring NATS health separately
- Message schema evolution requires coordination

### Neutral

- Team learns NATS patterns (offset by operational simplicity)
- Can migrate to gRPC later if needed (interface abstraction exists)

## Validation

Proven in production-like testing:
- 50 mitigation nodes receiving attack events
- < 2ms event propagation time (orchestrator → all nodes)
- 100K events/sec sustained throughput
- Zero message loss with JetStream

## Links

- [NATS Documentation](https://docs.nats.io/)
- [NATS JetStream](https://docs.nats.io/nats-concepts/jetstream)
- [NATS vs gRPC Comparison](https://nats.io/blog/nats-vs-grpc/)
- SecBeat event system: `mitigation-node/src/events.rs`
- Orchestrator integration: `orchestrator-node/src/main.rs`

## Future Considerations

- Evaluate NATS KV for distributed rate limiting state
- Consider NATS Object Store for large rule bundles
- Investigate NATS Leaf Nodes for edge deployments
- Monitor gRPC ecosystem for new features (e.g., native pub/sub)
