// CRDT (Conflict-free Replicated Data Type) Implementation
//
// This module implements G-Counter (Grow-only Counter) for distributed
// rate limiting across multiple mitigation nodes.
//
// G-Counter Properties:
// - Commutative: merge(A, B) = merge(B, A)
// - Associative: merge(merge(A, B), C) = merge(A, merge(B, C))
// - Idempotent: merge(A, A) = A
// - Convergent: All nodes eventually converge to same state

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Node identifier for CRDT operations
pub type NodeId = Uuid;

/// G-Counter: Grow-only Counter (State-based CRDT)
///
/// A distributed counter that can only increment. Each node maintains
/// its own counter value, and the global value is the sum of all counters.
///
/// # Theory
///
/// G-Counter is a state-based CRDT where:
/// - State S = {counts: Map<NodeId, u64>}
/// - Increment: counts[local_node] += delta
/// - Merge: counts[k] = max(local[k], remote[k]) for all k
/// - Value: sum(counts.values())
///
/// # Example
///
/// ```text
/// Node A: {A: 10, B: 0, C: 0} → value = 10
/// Node B: {A: 5,  B: 20, C: 0} → value = 25
/// merge(A, B) = {A: 10, B: 20, C: 0} → value = 30
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GCounter {
    /// Map of node_id → count for that node
    /// Only the local node increments its own counter
    counts: HashMap<NodeId, u64>,
}

impl GCounter {
    /// Create a new empty G-Counter
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    /// Create a G-Counter with initial nodes
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            counts: HashMap::with_capacity(capacity),
        }
    }

    /// Increment the counter for a specific node
    ///
    /// # Arguments
    /// * `node_id` - The node performing the increment
    /// * `delta` - Amount to increment by
    ///
    /// # Example
    /// ```
    /// use mitigation_node::distributed::crdt::{GCounter, NodeId};
    /// use uuid::Uuid;
    /// 
    /// let node_id = NodeId(Uuid::new_v4());
    /// let mut counter = GCounter::new();
    /// counter.inc(node_id, 10);
    /// counter.inc(node_id, 5);
    /// assert_eq!(counter.get(node_id), 15);
    /// ```
    pub fn inc(&mut self, node_id: NodeId, delta: u64) {
        *self.counts.entry(node_id).or_insert(0) += delta;
    }

    /// Get the count for a specific node
    pub fn get(&self, node_id: NodeId) -> u64 {
        self.counts.get(&node_id).copied().unwrap_or(0)
    }

    /// Merge another G-Counter into this one
    ///
    /// For each node in the other counter, take the maximum value.
    /// This ensures eventual consistency across all nodes.
    ///
    /// # Arguments
    /// * `other` - The remote G-Counter to merge
    ///
    /// # Merge Semantics
    /// ```text
    /// local[k] = max(local[k], remote[k]) for all k
    /// ```
    ///
    /// # Example
    /// ```
    /// use mitigation_node::distributed::crdt::{GCounter, NodeId};
    /// use uuid::Uuid;
    /// 
    /// let node_a = NodeId(Uuid::new_v4());
    /// let node_b = NodeId(Uuid::new_v4());
    /// 
    /// let mut counter_a = GCounter::new();
    /// counter_a.inc(node_a, 10);
    ///
    /// let mut counter_b = GCounter::new();
    /// counter_b.inc(node_b, 20);
    ///
    /// counter_a.merge(&counter_b);
    /// assert_eq!(counter_a.value(), 30); // 10 + 20
    /// ```
    pub fn merge(&mut self, other: &GCounter) {
        for (&node_id, &remote_count) in &other.counts {
            let local_count = self.counts.entry(node_id).or_insert(0);
            *local_count = (*local_count).max(remote_count);
        }
    }

    /// Get the global value (sum of all node counters)
    ///
    /// This is the observable value of the distributed counter.
    ///
    /// # Returns
    /// Sum of all node counts
    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    /// Get the number of nodes in this counter
    pub fn node_count(&self) -> usize {
        self.counts.len()
    }

    /// Check if the counter is empty
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// Reset the counter (for testing only)
    #[cfg(test)]
    pub fn reset(&mut self) {
        self.counts.clear();
    }

    /// Get a reference to the internal counts map
    pub fn counts(&self) -> &HashMap<NodeId, u64> {
        &self.counts
    }

    /// Create a delta (difference) between two counters
    ///
    /// This is useful for delta-based synchronization to reduce
    /// bandwidth usage by only sending changes.
    ///
    /// # Arguments
    /// * `baseline` - The previous state to compare against
    ///
    /// # Returns
    /// A new G-Counter containing only the changes
    pub fn delta(&self, baseline: &GCounter) -> GCounter {
        let mut delta = GCounter::new();
        
        for (&node_id, &current_count) in &self.counts {
            let baseline_count = baseline.get(node_id);
            if current_count > baseline_count {
                delta.counts.insert(node_id, current_count);
            }
        }
        
        delta
    }

    /// Apply a delta to this counter
    ///
    /// This is equivalent to merge but optimized for delta updates.
    pub fn apply_delta(&mut self, delta: &GCounter) {
        self.merge(delta);
    }
}

impl Default for GCounter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PN-Counter: Positive-Negative Counter (Future Enhancement)
// ============================================================================

/// PN-Counter: Counter that can increment and decrement
///
/// Composed of two G-Counters: one for increments (P) and one for decrements (N).
/// Value = P.value() - N.value()
///
/// NOTE: Currently not used, but included for future distributed quota management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PNCounter {
    /// Positive increments
    increments: GCounter,
    /// Negative decrements
    decrements: GCounter,
}

impl PNCounter {
    pub fn new() -> Self {
        Self {
            increments: GCounter::new(),
            decrements: GCounter::new(),
        }
    }

    pub fn inc(&mut self, node_id: NodeId, delta: u64) {
        self.increments.inc(node_id, delta);
    }

    pub fn dec(&mut self, node_id: NodeId, delta: u64) {
        self.decrements.inc(node_id, delta);
    }

    pub fn merge(&mut self, other: &PNCounter) {
        self.increments.merge(&other.increments);
        self.decrements.merge(&other.decrements);
    }

    pub fn value(&self) -> i64 {
        let pos = self.increments.value() as i64;
        let neg = self.decrements.value() as i64;
        pos - neg
    }
}

impl Default for PNCounter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn node_a() -> NodeId {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn node_b() -> NodeId {
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
    }

    fn node_c() -> NodeId {
        Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap()
    }

    #[test]
    fn test_gcounter_increment() {
        let mut counter = GCounter::new();
        
        counter.inc(node_a(), 10);
        assert_eq!(counter.get(node_a()), 10);
        assert_eq!(counter.value(), 10);

        counter.inc(node_a(), 5);
        assert_eq!(counter.get(node_a()), 15);
        assert_eq!(counter.value(), 15);
    }

    #[test]
    fn test_gcounter_multiple_nodes() {
        let mut counter = GCounter::new();
        
        counter.inc(node_a(), 10);
        counter.inc(node_b(), 20);
        counter.inc(node_c(), 30);

        assert_eq!(counter.get(node_a()), 10);
        assert_eq!(counter.get(node_b()), 20);
        assert_eq!(counter.get(node_c()), 30);
        assert_eq!(counter.value(), 60);
        assert_eq!(counter.node_count(), 3);
    }

    #[test]
    fn test_gcounter_merge_disjoint() {
        let mut counter_a = GCounter::new();
        counter_a.inc(node_a(), 10);

        let mut counter_b = GCounter::new();
        counter_b.inc(node_b(), 20);

        counter_a.merge(&counter_b);

        assert_eq!(counter_a.get(node_a()), 10);
        assert_eq!(counter_a.get(node_b()), 20);
        assert_eq!(counter_a.value(), 30);
    }

    #[test]
    fn test_gcounter_merge_overlapping() {
        let mut counter_a = GCounter::new();
        counter_a.inc(node_a(), 10);
        counter_a.inc(node_b(), 5);

        let mut counter_b = GCounter::new();
        counter_b.inc(node_a(), 8);  // Less than counter_a
        counter_b.inc(node_b(), 15); // More than counter_a

        counter_a.merge(&counter_b);

        // Should take max for each node
        assert_eq!(counter_a.get(node_a()), 10); // max(10, 8)
        assert_eq!(counter_a.get(node_b()), 15); // max(5, 15)
        assert_eq!(counter_a.value(), 25);
    }

    #[test]
    fn test_gcounter_merge_commutative() {
        let mut counter_a1 = GCounter::new();
        counter_a1.inc(node_a(), 10);

        let mut counter_b1 = GCounter::new();
        counter_b1.inc(node_b(), 20);

        let mut counter_a2 = counter_a1.clone();
        let counter_b2 = counter_b1.clone();

        // merge(A, B)
        counter_a1.merge(&counter_b1);

        // merge(B, A)
        counter_b1.merge(&counter_a2);

        // Should be equal (commutative)
        assert_eq!(counter_a1, counter_b1);
    }

    #[test]
    fn test_gcounter_merge_associative() {
        let mut counter_a = GCounter::new();
        counter_a.inc(node_a(), 10);

        let mut counter_b = GCounter::new();
        counter_b.inc(node_b(), 20);

        let mut counter_c = GCounter::new();
        counter_c.inc(node_c(), 30);

        // merge(merge(A, B), C)
        let mut result1 = counter_a.clone();
        result1.merge(&counter_b);
        result1.merge(&counter_c);

        // merge(A, merge(B, C))
        let mut result2 = counter_a.clone();
        let mut bc = counter_b.clone();
        bc.merge(&counter_c);
        result2.merge(&bc);

        // Should be equal (associative)
        assert_eq!(result1, result2);
        assert_eq!(result1.value(), 60);
    }

    #[test]
    fn test_gcounter_merge_idempotent() {
        let mut counter_a = GCounter::new();
        counter_a.inc(node_a(), 10);
        counter_a.inc(node_b(), 20);

        let counter_copy = counter_a.clone();

        // merge(A, A) should equal A
        counter_a.merge(&counter_copy);

        assert_eq!(counter_a, counter_copy);
        assert_eq!(counter_a.value(), 30);
    }

    #[test]
    fn test_gcounter_delta() {
        let mut baseline = GCounter::new();
        baseline.inc(node_a(), 10);
        baseline.inc(node_b(), 20);

        let mut current = baseline.clone();
        current.inc(node_a(), 5);  // Now 15
        current.inc(node_c(), 30); // New node

        let delta = current.delta(&baseline);

        // Delta should only contain changes
        assert_eq!(delta.get(node_a()), 15); // Changed
        assert_eq!(delta.get(node_b()), 0);  // Unchanged (not in delta)
        assert_eq!(delta.get(node_c()), 30); // New

        // Apply delta to baseline should equal current
        let mut reconstructed = baseline.clone();
        reconstructed.apply_delta(&delta);
        assert_eq!(reconstructed, current);
    }

    #[test]
    fn test_gcounter_concurrent_increments() {
        // Simulate concurrent increments from multiple nodes
        let mut node_a_view = GCounter::new();
        let mut node_b_view = GCounter::new();

        // Node A increments locally
        node_a_view.inc(node_a(), 10);

        // Node B increments locally (concurrent)
        node_b_view.inc(node_b(), 20);

        // Nodes exchange state
        let a_copy = node_a_view.clone();
        let b_copy = node_b_view.clone();

        node_a_view.merge(&b_copy);
        node_b_view.merge(&a_copy);

        // Both nodes should converge to same value
        assert_eq!(node_a_view.value(), 30);
        assert_eq!(node_b_view.value(), 30);
        assert_eq!(node_a_view, node_b_view);
    }

    #[test]
    fn test_pncounter_basic() {
        let mut counter = PNCounter::new();

        counter.inc(node_a(), 50);
        counter.dec(node_a(), 20);

        assert_eq!(counter.value(), 30);
    }

    #[test]
    fn test_pncounter_merge() {
        let mut counter_a = PNCounter::new();
        counter_a.inc(node_a(), 50);
        counter_a.dec(node_a(), 10);

        let mut counter_b = PNCounter::new();
        counter_b.inc(node_b(), 30);
        counter_b.dec(node_b(), 5);

        counter_a.merge(&counter_b);

        // (50 - 10) + (30 - 5) = 40 + 25 = 65
        assert_eq!(counter_a.value(), 65);
    }
}
