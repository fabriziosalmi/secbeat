// SecBeat Common - Shared types between kernel (eBPF) and userspace
// This crate is no_std compatible for use in eBPF programs

#![no_std]

/// IP address representation for eBPF programs
/// Using u32 for IPv4 (network byte order)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct IpAddr {
    pub addr: u32,
}

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
