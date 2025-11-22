#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::HashMap,
    programs::XdpContext,
};
use aya_log_ebpf::info;
use core::mem;
use secbeat_common::{BlockEntry, MAX_BLOCKLIST_ENTRIES};

// Ethernet header constants
const ETH_P_IP: u16 = 0x0800; // IPv4 protocol (big-endian)
const ETH_HLEN: usize = 14; // Ethernet header length

// IPv4 header offset
const IP_SADDR_OFFSET: usize = 12; // Source address offset in IPv4 header

/// Blocklist map: IP address (u32) -> BlockEntry
/// Userspace populates this map, kernel reads it for O(1) lookups
#[map]
static BLOCKLIST: HashMap<u32, BlockEntry> = HashMap::with_max_entries(MAX_BLOCKLIST_ENTRIES, 0);

/// SecBeat XDP Program - Phase 2.2: The Bouncer
/// High-performance packet filtering using eBPF/XDP
///
/// This program:
/// 1. Parses Ethernet and IPv4 headers
/// 2. Extracts source IP address
/// 3. Checks IP against kernel blocklist (O(1) lookup)
/// 4. Drops malicious traffic instantly (before network stack)
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
        return Ok(xdp_action::XDP_PASS); // Malformed packet, let stack handle
    }

    // Parse Ethernet header
    let eth_proto = unsafe {
        let proto_ptr = (data + 12) as *const u16;
        u16::from_be(*proto_ptr) // Read EtherType in big-endian
    };

    // Check if this is an IPv4 packet
    if eth_proto != ETH_P_IP {
        return Ok(xdp_action::XDP_PASS); // Not IPv4, pass through
    }

    // Bounds check: ensure we have IPv4 header (minimum 20 bytes)
    if data + ETH_HLEN + 20 > data_end {
        return Ok(xdp_action::XDP_PASS); // Truncated packet
    }

    // Extract source IP address (network byte order)
    let src_ip = unsafe {
        let ip_hdr = data + ETH_HLEN;
        let saddr_ptr = (ip_hdr + IP_SADDR_OFFSET) as *const u32;
        *saddr_ptr // This is in network byte order (big-endian)
    };

    // Check if source IP is in blocklist
    // NOTE: The map key should match the byte order we're using here (network/big-endian)
    if let Some(_entry) = unsafe { BLOCKLIST.get(&src_ip) } {
        info!(&ctx, "🚫 DROP {:i}", src_ip);
        return Ok(xdp_action::XDP_DROP);
    }

    // IP not blocked, allow packet
    Ok(xdp_action::XDP_PASS)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

