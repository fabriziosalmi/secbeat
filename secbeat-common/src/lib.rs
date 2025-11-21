// SecBeat Common - Shared types between kernel (eBPF) and userspace
// This crate is no_std compatible for use in eBPF programs

#![no_std]

/// Maximum number of IPs in the blocklist
/// This limits memory usage in the kernel (10240 * 8 bytes = ~80KB)
pub const MAX_BLOCKLIST_ENTRIES: u32 = 10240;

/// IP address representation for eBPF programs
/// Using u32 for IPv4 (network byte order)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IpAddr {
    pub addr: u32,
}

/// Blocklist entry metadata
/// Stores information about blocked IPs
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockEntry {
    /// Timestamp when IP was blocked (Unix epoch)
    pub blocked_at: u64,
    /// Number of packets dropped from this IP
    pub hit_count: u32,
    /// Reserved for future use
    pub flags: u32,
}

// Implement Pod for userspace Aya
#[cfg(feature = "user")]
unsafe impl aya::Pod for BlockEntry {}

/// Statistics structure shared between eBPF and userspace
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PacketStats {
    /// Total packets processed
    pub packets_total: u64,
    /// Packets passed through
    pub packets_passed: u64,
    /// Packets dropped
    pub packets_dropped: u64,
}

impl PacketStats {
    pub const fn new() -> Self {
        Self {
            packets_total: 0,
            packets_passed: 0,
            packets_dropped: 0,
        }
    }
}
