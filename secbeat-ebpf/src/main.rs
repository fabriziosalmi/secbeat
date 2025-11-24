#![no_std]
#![no_main]

mod csum;
mod cookie;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{HashMap, PerCpuArray},
    programs::XdpContext,
};
use aya_log_ebpf::info;
use secbeat_common::{
    BlockEntry, Ipv4Hdr, TcpHdr, 
    MAX_BLOCKLIST_ENTRIES, STAT_PASS, STAT_DROP,
    ETH_P_IP, ETH_HLEN, IPPROTO_TCP, TCP_SYN, TCP_ACK,
};

// IPv4 header offsets
const IP_SADDR_OFFSET: usize = 12; // Source address offset in IPv4 header

/// Blocklist map: IP address (u32) -> BlockEntry
/// Userspace populates this map, kernel reads it for O(1) lookups
#[map]
static BLOCKLIST: HashMap<u32, BlockEntry> = HashMap::with_max_entries(MAX_BLOCKLIST_ENTRIES, 0);

/// Statistics map: Per-CPU counters for PASS and DROP actions
/// Index 0: Packets passed (XDP_PASS)
/// Index 1: Packets dropped (XDP_DROP)
#[map]
static STATS: PerCpuArray<u64> = PerCpuArray::with_max_entries(2, 0);

/// SecBeat XDP Program - Phases 2.2-2.4: The Bouncer + SYN Flood Protection
/// High-performance packet filtering and SYN flood mitigation using eBPF/XDP
///
/// This program implements:
/// 1. IP-based blocklist filtering (Chapter 2.2)
/// 2. Packet statistics tracking (Chapter 2.3)
/// 3. Stateless SYN cookie generation for SYN flood protection (Chapter 2.4)
///
/// SYN Flood Protection:
/// - Intercepts SYN packets before they reach the kernel
/// - Generates cryptographic cookies using connection 4-tuple
/// - Responds with SYN-ACK containing cookie as sequence number (XDP_TX)
/// - Drops original SYN to prevent kernel memory allocation
/// - ACK packets are passed to kernel for normal processing
#[xdp]
pub fn secbeat_xdp(ctx: XdpContext) -> u32 {
    match try_secbeat_xdp(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

fn try_secbeat_xdp(ctx: XdpContext) -> Result<u32, ()> {
    let data = ctx.data();
    let data_end = ctx.data_end();

    // Bounds check: ensure we have at least Ethernet header
    if data + ETH_HLEN > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // Parse Ethernet header
    let eth_proto = unsafe {
        let proto_ptr = (data + 12) as *const u16;
        u16::from_be(*proto_ptr)
    };

    // Check if this is an IPv4 packet
    if eth_proto != ETH_P_IP {
        return Ok(xdp_action::XDP_PASS);
    }

    // Bounds check: ensure we have IPv4 header (minimum 20 bytes)
    if data + ETH_HLEN + 20 > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // Get IP header pointer
    let ip_hdr_ptr = (data + ETH_HLEN) as *mut Ipv4Hdr;
    let ip_hdr = unsafe { &*ip_hdr_ptr };

    // Check if this is a TCP packet
    if ip_hdr.protocol != IPPROTO_TCP {
        // Not TCP, fall through to blocklist check
        return check_blocklist_and_pass(&ctx, ip_hdr.saddr);
    }

    // Bounds check: ensure we have TCP header
    let ip_hlen = ip_hdr.ihl() as usize;
    let tcp_offset = ETH_HLEN + ip_hlen;
    
    if data + tcp_offset + 20 > data_end {
        return Ok(xdp_action::XDP_PASS);
    }

    // Get TCP header pointer
    let tcp_hdr_ptr = (data + tcp_offset) as *mut TcpHdr;
    let tcp_hdr = unsafe { &*tcp_hdr_ptr };

    // SYN Flood Protection: Handle SYN packets
    if tcp_hdr.is_syn() && !tcp_hdr.is_ack() {
        // This is a SYN packet - generate SYN cookie and respond with SYN-ACK
        return handle_syn_packet(&ctx, data, data_end, ip_hdr_ptr, tcp_hdr_ptr, ip_hlen);
    }

    // Handle ACK packets (potential cookie verification)
    if tcp_hdr.is_ack() && !tcp_hdr.is_syn() {
        // For now, just pass ACK packets to kernel
        // In a full implementation, we'd verify the cookie here
        return check_blocklist_and_pass(&ctx, ip_hdr.saddr);
    }

    // For all other TCP packets, check blocklist
    check_blocklist_and_pass(&ctx, ip_hdr.saddr)
}

/// Check if IP is in blocklist and update stats
#[inline(always)]
fn check_blocklist_and_pass(ctx: &XdpContext, src_ip: u32) -> Result<u32, ()> {
    if let Some(_entry) = unsafe { BLOCKLIST.get(&src_ip) } {
        info!(ctx, "🚫 DROP {:i}", src_ip);
        
        if let Some(counter) = unsafe { STATS.get_ptr_mut(STAT_DROP) } {
            unsafe { *counter += 1; }
        }
        
        return Ok(xdp_action::XDP_DROP);
    }

    if let Some(counter) = unsafe { STATS.get_ptr_mut(STAT_PASS) } {
        unsafe { *counter += 1; }
    }

    Ok(xdp_action::XDP_PASS)
}

/// Handle SYN packet - generate SYN cookie and send SYN-ACK
#[inline(never)] // Too complex to inline
fn handle_syn_packet(
    ctx: &XdpContext,
    data: usize,
    data_end: usize,
    ip_hdr: *mut Ipv4Hdr,
    tcp_hdr: *mut TcpHdr,
    ip_hlen: usize,
) -> Result<u32, ()> {
    // Read current values before modification
    let (saddr, daddr, sport, dport, seq) = unsafe {
        let ip = &*ip_hdr;
        let tcp = &*tcp_hdr;
        (ip.saddr, ip.daddr, tcp.source, tcp.dest, tcp.seq)
    };

    info!(ctx, "🍪 SYN from {:i}:{} - generating cookie", saddr, u16::from_be(sport));

    // Generate SYN cookie
    let cookie = cookie::gen_cookie(saddr, daddr, sport, dport, seq);

    // === SWAP ETHERNET ADDRESSES ===
    // Get MAC addresses and swap them
    let eth_hdr = data as *mut [u8; 12];
    unsafe {
        let temp_mac = [(*eth_hdr)[0], (*eth_hdr)[1], (*eth_hdr)[2], 
                       (*eth_hdr)[3], (*eth_hdr)[4], (*eth_hdr)[5]];
        (*eth_hdr)[0] = (*eth_hdr)[6];
        (*eth_hdr)[1] = (*eth_hdr)[7];
        (*eth_hdr)[2] = (*eth_hdr)[8];
        (*eth_hdr)[3] = (*eth_hdr)[9];
        (*eth_hdr)[4] = (*eth_hdr)[10];
        (*eth_hdr)[5] = (*eth_hdr)[11];
        (*eth_hdr)[6] = temp_mac[0];
        (*eth_hdr)[7] = temp_mac[1];
        (*eth_hdr)[8] = temp_mac[2];
        (*eth_hdr)[9] = temp_mac[3];
        (*eth_hdr)[10] = temp_mac[4];
        (*eth_hdr)[11] = temp_mac[5];
    }

    // === MODIFY IP HEADER ===
    unsafe {
        let ip = &mut *ip_hdr;
        // Swap IP addresses
        ip.saddr = daddr;
        ip.daddr = saddr;
        // Keep TTL reasonable
        ip.ttl = 64;
        // Zero checksum before recalculating
        ip.check = 0;
    }

    // === MODIFY TCP HEADER ===
    unsafe {
        let tcp = &mut *tcp_hdr;
        // Swap ports
        tcp.source = dport;
        tcp.dest = sport;
        // Set sequence number to our cookie
        tcp.seq = u32::to_be(cookie);
        // Set ACK number to client's seq + 1
        tcp.ack_seq = u32::to_be(u32::from_be(seq).wrapping_add(1));
        // Set flags to SYN | ACK (with 5 * 4 = 20 byte header)
        tcp.set_flags(TCP_SYN | TCP_ACK, 20);
        // Keep window size reasonable
        tcp.window = u16::to_be(65535);
        // Zero checksum before recalculating
        tcp.check = 0;
    }

    // === RECALCULATE CHECKSUMS ===
    // IP checksum
    let ip_csum = csum::ipv4_csum(ip_hdr as *const u8, ip_hlen);
    unsafe {
        (*ip_hdr).check = ip_csum;
    }

    // TCP checksum (need to calculate over header only, no data)
    let tcp_offset = ETH_HLEN + ip_hlen;
    let tcp_len = 20u16; // Minimum TCP header size
    let tcp_csum = csum::tcp_csum(daddr, saddr, tcp_hdr as *const u8, tcp_len);
    unsafe {
        (*tcp_hdr).check = tcp_csum;
    }

    info!(ctx, "🍪 TX SYN-ACK to {:i}:{} (cookie: {})", daddr, u16::from_be(dport), cookie);

    // Send packet back out the same interface
    Ok(xdp_action::XDP_TX)
}
