// SecBeat Common - Shared types between kernel (eBPF) and userspace
// This crate is no_std compatible for use in eBPF programs

#![no_std]

/// Maximum number of IPs in the blocklist
/// This limits memory usage in the kernel (10240 * 8 bytes = ~80KB)
pub const MAX_BLOCKLIST_ENTRIES: u32 = 10240;

/// Statistics indices for the STATS map
/// Using constants for array indexing in eBPF
pub const STAT_PASS: u32 = 0;
pub const STAT_DROP: u32 = 1;
pub const STAT_ARRAY_SIZE: u32 = 2;

// Network protocol constants
/// Ethernet Protocol ID for IPv4
pub const ETH_P_IP: u16 = 0x0800;
/// IP Protocol number for TCP
pub const IPPROTO_TCP: u8 = 6;

// TCP flags (from TCP header flags byte)
pub const TCP_FIN: u8 = 0x01;
pub const TCP_SYN: u8 = 0x02;
pub const TCP_RST: u8 = 0x04;
pub const TCP_PSH: u8 = 0x08;
pub const TCP_ACK: u8 = 0x10;
pub const TCP_URG: u8 = 0x20;

// Protocol header sizes
pub const ETH_HLEN: usize = 14;
pub const IPV4_HLEN_MIN: usize = 20;
pub const TCP_HLEN_MIN: usize = 20;

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
#[derive(Clone, Copy, Debug)]
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

impl Default for PacketStats {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP Header structure (simplified for eBPF use)
/// All multi-byte fields are in network byte order (big-endian)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TcpHdr {
    pub source: u16,      // Source port
    pub dest: u16,        // Destination port
    pub seq: u32,         // Sequence number
    pub ack_seq: u32,     // Acknowledgment number
    pub _bitfield: u16,   // Data offset (4 bits) + Reserved (3 bits) + Flags (9 bits)
    pub window: u16,      // Window size
    pub check: u16,       // Checksum
    pub urg_ptr: u16,     // Urgent pointer
}

impl TcpHdr {
    /// Get TCP data offset in bytes (header length)
    #[inline(always)]
    pub fn data_offset(&self) -> u8 {
        ((u16::from_be(self._bitfield) >> 12) & 0x0F) as u8 * 4
    }

    /// Get TCP flags byte
    #[inline(always)]
    pub fn flags(&self) -> u8 {
        (u16::from_be(self._bitfield) & 0x00FF) as u8
    }

    /// Set TCP flags
    #[inline(always)]
    pub fn set_flags(&mut self, flags: u8, data_offset: u8) {
        let doff = ((data_offset / 4) as u16) << 12;
        self._bitfield = u16::to_be(doff | (flags as u16));
    }

    /// Check if SYN flag is set
    #[inline(always)]
    pub fn is_syn(&self) -> bool {
        self.flags() & TCP_SYN != 0
    }

    /// Check if ACK flag is set
    #[inline(always)]
    pub fn is_ack(&self) -> bool {
        self.flags() & TCP_ACK != 0
    }
}

/// IPv4 Header structure (simplified for eBPF use)
/// All multi-byte fields are in network byte order (big-endian)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Ipv4Hdr {
    pub _bitfield: u8,    // Version (4 bits) + IHL (4 bits)
    pub tos: u8,          // Type of Service
    pub tot_len: u16,     // Total Length
    pub id: u16,          // Identification
    pub frag_off: u16,    // Fragment Offset
    pub ttl: u8,          // Time to Live
    pub protocol: u8,     // Protocol
    pub check: u16,       // Header Checksum
    pub saddr: u32,       // Source Address
    pub daddr: u32,       // Destination Address
}

impl Ipv4Hdr {
    /// Get IP header length in bytes
    #[inline(always)]
    pub fn ihl(&self) -> u8 {
        (self._bitfield & 0x0F) * 4
    }

    /// Set IP header length (in 32-bit words)
    #[inline(always)]
    pub fn set_ihl(&mut self, ihl: u8) {
        self._bitfield = (self._bitfield & 0xF0) | (ihl & 0x0F);
    }
}
