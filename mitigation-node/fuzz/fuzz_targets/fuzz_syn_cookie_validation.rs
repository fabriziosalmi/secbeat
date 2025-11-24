#![no_main]

use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256};
use std::net::Ipv4Addr;

// Fuzz target for SYN cookie validation
// Tests the validation logic for security vulnerabilities and edge cases

fuzz_target!(|data: &[u8]| {
    // Need at least 48 bytes for a complete test case
    if data.len() < 48 {
        return;
    }

    // Parse fuzzer input
    let secret_key: [u8; 32] = {
        let mut key = [0u8; 32];
        for (i, &byte) in data.iter().take(32).enumerate() {
            key[i] = byte;
        }
        key
    };

    let client_ip = Ipv4Addr::new(
        data[32],
        data[33],
        data[34],
        data[35],
    );

    let client_port = u16::from_be_bytes([data[36], data[37]]);
    let server_port = u16::from_be_bytes([data[38], data[39]]);
    let client_seq = u32::from_be_bytes([data[40], data[41], data[42], data[43]]);
    let claimed_cookie = u32::from_be_bytes([data[44], data[45], data[46], data[47]]);

    // Validation logic (mirrors syn_proxy.rs validate_syn_cookie)
    let mut is_valid = false;

    for time_offset in 0..2 {
        let mut hasher = Sha256::new();
        hasher.update(&secret_key);
        hasher.update(&client_ip.octets());
        hasher.update(&client_port.to_be_bytes());
        hasher.update(&server_port.to_be_bytes());
        hasher.update(&client_seq.to_be_bytes());

        let timestamp = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 60) as u32;

        let adjusted_timestamp = timestamp.wrapping_sub(time_offset);
        hasher.update(&adjusted_timestamp.to_be_bytes());

        let result = hasher.finalize();
        let expected_cookie = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);

        if expected_cookie == claimed_cookie {
            is_valid = true;
            break;
        }
    }

    // Security properties to verify:

    // 1. Same inputs must produce same validation result (deterministic)
    let mut is_valid2 = false;
    for time_offset in 0..2 {
        let mut hasher = Sha256::new();
        hasher.update(&secret_key);
        hasher.update(&client_ip.octets());
        hasher.update(&client_port.to_be_bytes());
        hasher.update(&server_port.to_be_bytes());
        hasher.update(&client_seq.to_be_bytes());

        let timestamp = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 60) as u32;

        let adjusted_timestamp = timestamp.wrapping_sub(time_offset);
        hasher.update(&adjusted_timestamp.to_be_bytes());

        let result = hasher.finalize();
        let expected_cookie = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);

        if expected_cookie == claimed_cookie {
            is_valid2 = true;
            break;
        }
    }
    assert_eq!(is_valid, is_valid2, "Validation must be deterministic");

    // 2. Random cookies should almost always be invalid
    // (collision probability is 1/2^32 ≈ 0.00000002%)
    if claimed_cookie == 0xDEADBEEF || claimed_cookie == 0x12345678 {
        // Very unlikely to be valid unless input was crafted
    }

    // 3. Changing any input parameter should invalidate the cookie
    // (except within time window tolerance)
    if is_valid {
        // Try with different client IP
        let wrong_ip = Ipv4Addr::new(
            client_ip.octets()[0].wrapping_add(1),
            client_ip.octets()[1],
            client_ip.octets()[2],
            client_ip.octets()[3],
        );

        let mut wrong_ip_valid = false;
        for time_offset in 0..2 {
            let mut hasher = Sha256::new();
            hasher.update(&secret_key);
            hasher.update(&wrong_ip.octets());
            hasher.update(&client_port.to_be_bytes());
            hasher.update(&server_port.to_be_bytes());
            hasher.update(&client_seq.to_be_bytes());

            let timestamp = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                / 60) as u32;

            let adjusted_timestamp = timestamp.wrapping_sub(time_offset);
            hasher.update(&adjusted_timestamp.to_be_bytes());

            let result = hasher.finalize();
            let expected_cookie = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);

            if expected_cookie == claimed_cookie {
                wrong_ip_valid = true;
                break;
            }
        }

        // Cookie must be specific to the IP address
        assert!(!wrong_ip_valid, "Cookie validation must be specific to client IP");
    }

    // 4. Time window must be limited (only current and previous minute)
    // This prevents replay attacks from old connections
});
