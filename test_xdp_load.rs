// Quick XDP loader for testing secbeat-ebpf
// Run with: cargo +nightly build --release && sudo target/release/test_xdp_load

use aya::programs::{Xdp, XdpFlags};
use aya::Ebpf;
use std::env;

fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = env::args().collect();
    let iface = args.get(1).map(|s| s.as_str()).unwrap_or("eth0");

    println!("🔧 Loading SecBeat XDP program on interface: {}", iface);

    // Load the compiled eBPF program
    let mut ebpf = Ebpf::load_file("target/bpf/secbeat-ebpf")?;
    println!("✅ eBPF program loaded from file");

    // Find the XDP program by name
    let program: &mut Xdp = ebpf.program_mut("secbeat_xdp").unwrap().try_into()?;
    println!("✅ Found XDP program: secbeat_xdp");

    // Load the program into the kernel
    program.load()?;
    println!("✅ Program loaded into kernel");

    // Attach to the network interface
    program.attach(iface, XdpFlags::default())?;
    println!("✅ XDP program attached to {}", iface);
    println!("");
    println!("✨ SecBeat XDP is now running!");
    println!("Press Ctrl+C to detach and exit...");

    // Keep running until Ctrl+C
    std::thread::park();

    Ok(())
}
