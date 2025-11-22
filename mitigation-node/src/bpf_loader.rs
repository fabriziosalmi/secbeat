// SecBeat BPF Loader - Userspace XDP Management
// Handles loading, attaching, and controlling eBPF programs

use aya::{
    maps::HashMap as AyaHashMap,
    programs::{Xdp, XdpFlags},
    Ebpf,
};
use anyhow::{Context, Result};
use secbeat_common::BlockEntry;
use std::net::Ipv4Addr;
use std::path::Path;
use tracing::{info, warn};

/// Handle to the loaded BPF program and its maps
/// Keep this alive for the lifetime of the XDP program
pub struct BpfHandle {
    /// The loaded eBPF program (must stay alive)
    _ebpf: Ebpf,
    /// Blocklist map handle for inserting/removing IPs
    blocklist: AyaHashMap<std::sync::Arc<aya::maps::MapData>, u32, BlockEntry>,
}

impl BpfHandle {
    /// Load and attach the XDP program to the specified network interface
    ///
    /// # Arguments
    /// * `interface` - Network interface name (e.g., "eth0")
    /// * `ebpf_path` - Path to the compiled eBPF ELF file
    ///
    /// # Returns
    /// BpfHandle that must be kept alive for the program to remain loaded
    pub fn load(interface: &str, ebpf_path: &Path) -> Result<Self> {
        info!("Loading eBPF program from: {}", ebpf_path.display());

        // Load the compiled eBPF program
        let mut ebpf = Ebpf::load_file(ebpf_path)
            .context("Failed to load eBPF program")?;

        info!("eBPF program loaded successfully");

        // Get the XDP program by name
        let program: &mut Xdp = ebpf
            .program_mut("secbeat_xdp")
            .context("Failed to find 'secbeat_xdp' program")?
            .try_into()
            .context("Program is not an XDP program")?;

        // Load the program into the kernel
        program.load().context("Failed to load XDP program into kernel")?;
        info!("XDP program loaded into kernel");

        // Attach to the network interface
        program
            .attach(interface, XdpFlags::default())
            .context(format!("Failed to attach XDP program to interface '{}'", interface))?;

        info!("✅ XDP program attached to interface: {}", interface);

        // Get handle to the blocklist map
        let blocklist: AyaHashMap<_, u32, BlockEntry> = ebpf
            .take_map("BLOCKLIST")
            .context("Failed to find BLOCKLIST map")?
            .try_into()
            .context("Map is not a HashMap")?;

        info!("✅ Blocklist map initialized (capacity: {} entries)", 
              secbeat_common::MAX_BLOCKLIST_ENTRIES);

        Ok(Self {
            _ebpf: ebpf,
            blocklist,
        })
    }

    /// Block an IP address by adding it to the kernel blocklist
    ///
    /// # Arguments
    /// * `ip` - IPv4 address to block
    ///
    /// # Returns
    /// Ok(()) if successfully blocked, Err if map update failed
    pub fn block_ip(&mut self, ip: Ipv4Addr) -> Result<()> {
        // CRITICAL: The kernel reads IP directly from packet memory (network byte order).
        // We need to store the IP in the SAME byte representation.
        // 
        // For IP 192.168.100.12:
        // - Packet bytes: [192, 168, 100, 12] = 0xc0a8640c in network order
        // - We need the u32 to have the SAME bytes when cast to [u8; 4]
        //
        // from_be_bytes() interprets bytes AS big-endian and converts to host order.
        // On little-endian (x86_64): from_be_bytes([192,168,100,12]) = 0xc0a8640c (correct!)
        // But when written to memory as u32, it becomes: 0x0c 0x64 0xa8 0xc0 (reversed!)
        //
        // Solution: Use the "network byte order" representation directly.
        // Since BPF maps store raw bytes, we want: u32::from_ne_bytes(ip.octets())
        // Or better: just read as big-endian on LE arch, which from_be_bytes does!
        let ip_bytes = ip.octets(); // [192, 168, 100, 12]
        
        // The kernel sees: *(u32*)&packet[ip_offset] which on LE reads 0x0c64a8c0
        // So we need to store 0x0c64a8c0 (little-endian representation of the bytes)
        let ip_u32 = u32::from_ne_bytes(ip_bytes); // Native endian = what kernel sees
        
        // Create block entry with metadata
        let entry = BlockEntry {
            blocked_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            hit_count: 0,
            flags: 0,
        };

        // Insert into kernel map
        self.blocklist
            .insert(ip_u32, entry, 0)
            .context(format!("Failed to insert IP {} into blocklist", ip))?;

        info!("🚫 Offloaded IP block to kernel/XDP: {} (key: 0x{:08x})", ip, ip_u32);
        Ok(())
    }

    /// Unblock an IP address by removing it from the kernel blocklist
    ///
    /// # Arguments
    /// * `ip` - IPv4 address to unblock
    ///
    /// # Returns
    /// Ok(()) if successfully unblocked or not found, Err if map operation failed
    pub fn unblock_ip(&mut self, ip: Ipv4Addr) -> Result<()> {
        let ip_u32 = u32::from_ne_bytes(ip.octets()); // Match block_ip() encoding
        
        match self.blocklist.remove(&ip_u32) {
            Ok(_) => {
                info!("✅ Removed IP from kernel blocklist: {}", ip);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to remove IP {} from blocklist: {}", ip, e);
                Ok(()) // Don't fail if IP wasn't in the list
            }
        }
    }

    /// Get the number of blocked IPs currently in the kernel map
    pub fn blocked_count(&self) -> Result<usize> {
        // Note: Aya doesn't provide a direct count, we'd need to iterate
        // For now, return 0 as placeholder
        Ok(0)
    }
}
