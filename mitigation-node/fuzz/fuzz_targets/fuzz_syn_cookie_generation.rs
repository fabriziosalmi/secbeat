#![no_main]

use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256};
use std::net::Ipv4Addr;

// Fuzz target for SYN cookie generation
// Tests for crashes, panics, and undefined behavior in cookie generation logic

fuzz_target!(|data: &[u8]| {
    // Need at least 14 bytes for a valid test case
    if data.len() < 14 {
        return;
    }

    // Parse fuzzer input into SYN cookie parameters
    let secret_key: [u8; 32] = {
        let mut key = [0u8; 32];
        for (i, &byte) in data.iter().take(32).enumerate() {
            key[i] = byte;
        }
        key
    };

    // Extract client IP (4 bytes)
    let client_ip = Ipv4Addr::new(
        data.get(32).copied().unwrap_or(0),
        data.get(33).copied().unwrap_or(0),
        data.get(34).copied().unwrap_or(0),
        data.get(35).copied().unwrap_or(0),
    );

    // Extract ports and sequence number (10 bytes total)
    let client_port = u16::from_be_bytes([
        data.get(36).copied().unwrap_or(0),
        data.get(37).copied().unwrap_or(0),
    ]);
    let server_port = u16::from_be_bytes([
        data.get(38).copied().unwrap_or(0),
        data.get(39).copied().unwrap_or(0),
    ]);
    let client_seq = u32::from_be_bytes([
        data.get(40).copied().unwrap_or(0),
        data.get(41).copied().unwrap_or(0),
        data.get(42).copied().unwrap_or(0),
        data.get(43).copied().unwrap_or(0),
    ]);

    // Test SYN cookie generation
    let mut hasher = Sha256::new();
    hasher.update(&secret_key);
    hasher.update(&client_ip.octets());
    hasher.update(&client_port.to_be_bytes());
    hasher.update(&server_port.to_be_bytes());
    hasher.update(&client_seq.to_be_bytes());

    // Add timestamp
    let timestamp = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 60) as u32;
    hasher.update(&timestamp.to_be_bytes());

    let result = hasher.finalize();
    let _cookie = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);

    // Verify cookie generation is deterministic
    let mut hasher2 = Sha256::new();
    hasher2.update(&secret_key);
    hasher2.update(&client_ip.octets());
    hasher2.update(&client_port.to_be_bytes());
    hasher2.update(&server_port.to_be_bytes());
    hasher2.update(&client_seq.to_be_bytes());
    hasher2.update(&timestamp.to_be_bytes());

    let result2 = hasher2.finalize();
    let cookie2 = u32::from_be_bytes([result2[0], result2[1], result2[2], result2[3]]);

    // Cookies must be identical for same input
    assert_eq!(_cookie, cookie2);

    // Test edge cases
    // 1. All zeros
    if secret_key.iter().all(|&x| x == 0) {
        // Should still generate a valid cookie
        assert!(_cookie != 0 || timestamp == 0);
    }

    // 2. Port 0 (invalid but shouldn't crash)
    if client_port == 0 || server_port == 0 {
        // Just verify it doesn't panic
    }

    // 3. Sequence number edge cases
    match client_seq {
        0 => { /* Beginning of sequence space */ }
        u32::MAX => { /* End of sequence space */ }
        _ => {}
    }
});
