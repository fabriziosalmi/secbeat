// Distributed coordination and state management
//
// This module provides distributed primitives for coordinating
// multiple mitigation nodes:
// - CRDTs for eventually consistent state
// - State synchronization via NATS
// - Global rate limiting

pub mod crdt;
pub mod state_sync;

pub use crdt::{GCounter, NodeId, PNCounter};
pub use state_sync::{StateManager, StateStats, StateSyncConfig, StateUpdate};
