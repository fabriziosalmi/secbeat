// SYN Cookie generation for stateless SYN flood mitigation
// Uses a fast hash function to generate cryptographic cookies

/// Secret key for cookie generation (should be rotated periodically in production)
/// For now, using a hardcoded value. In production, this could be loaded from a map
const COOKIE_SECRET: u32 = 0xDEADBEEF;

/// Generate a SYN cookie based on connection 4-tuple
///
/// The cookie is generated using a fast hash of:
/// - Source IP address
/// - Destination IP address  
/// - Source port
/// - Destination port
/// - Secret key
/// - Initial sequence number (from SYN packet)
///
/// This creates a cryptographically-bound cookie that can be verified
/// when the client sends back the ACK.
///
/// # Arguments
/// * `saddr` - Source IP address (network byte order)
/// * `daddr` - Destination IP address (network byte order)
/// * `sport` - Source port (network byte order)
/// * `dport` - Destination port (network byte order)
/// * `seq` - Initial sequence number from client SYN
///
/// # Returns
/// 32-bit cookie value to use as server's sequence number
#[inline(always)]
pub fn gen_cookie(saddr: u32, daddr: u32, sport: u16, dport: u16, seq: u32) -> u32 {
    // Use Jenkins hash algorithm (fast and suitable for eBPF)
    let mut hash = COOKIE_SECRET;
    
    // Mix in source address
    hash = hash.wrapping_add(saddr);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    
    // Mix in destination address
    hash = hash.wrapping_add(daddr);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    
    // Mix in ports (combine into single u32)
    let ports = ((sport as u32) << 16) | (dport as u32);
    hash = hash.wrapping_add(ports);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    
    // Mix in client's sequence number
    hash = hash.wrapping_add(seq);
    hash = hash.wrapping_add(hash << 10);
    hash ^= hash >> 6;
    
    // Final avalanche
    hash = hash.wrapping_add(hash << 3);
    hash ^= hash >> 11;
    hash = hash.wrapping_add(hash << 15);
    
    hash
}

/// Verify a SYN cookie
///
/// When we receive an ACK, we need to verify that the acknowledgment number
/// matches the cookie we sent + 1.
///
/// # Arguments
/// * `saddr` - Source IP address (network byte order)
/// * `daddr` - Destination IP address (network byte order)
/// * `sport` - Source port (network byte order)
/// * `dport` - Destination port (network byte order)
/// * `recv_seq` - Sequence number from the ACK packet
/// * `recv_ack` - Acknowledgment number from the ACK packet
///
/// # Returns
/// true if cookie is valid, false otherwise
#[inline(always)]
pub fn verify_cookie(
    saddr: u32,
    daddr: u32,
    sport: u16,
    dport: u16,
    recv_seq: u32,
    recv_ack: u32,
) -> bool {
    // The ACK should be our cookie + 1
    // We need to reconstruct what the original SYN seq was
    // This is tricky - for now, we'll do a simplified check
    
    // The client's original seq should be recv_seq - 1
    let original_seq = recv_seq.wrapping_sub(1);
    
    // Regenerate what our cookie should have been
    let expected_cookie = gen_cookie(daddr, saddr, dport, sport, original_seq);
    
    // The ACK should be our cookie + 1
    let expected_ack = expected_cookie.wrapping_add(1);
    
    recv_ack == expected_ack
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_generation() {
        let saddr = u32::from_be_bytes([192, 168, 1, 100]);
        let daddr = u32::from_be_bytes([192, 168, 1, 1]);
        let sport = u16::to_be(12345);
        let dport = u16::to_be(443);
        let seq = 0x12345678;

        let cookie1 = gen_cookie(saddr, daddr, sport, dport, seq);
        let cookie2 = gen_cookie(saddr, daddr, sport, dport, seq);

        // Same inputs should produce same cookie
        assert_eq!(cookie1, cookie2);

        // Different inputs should produce different cookie
        let cookie3 = gen_cookie(saddr, daddr, sport, dport, seq + 1);
        assert_ne!(cookie1, cookie3);
    }

    #[test]
    fn test_cookie_verification() {
        let saddr = u32::from_be_bytes([192, 168, 1, 100]);
        let daddr = u32::from_be_bytes([192, 168, 1, 1]);
        let sport = u16::to_be(12345);
        let dport = u16::to_be(443);
        let client_seq = 0x12345678;

        // Generate cookie for SYN
        let server_seq = gen_cookie(saddr, daddr, sport, dport, client_seq);

        // Client sends ACK with seq = client_seq + 1, ack = server_seq + 1
        let ack_seq = client_seq.wrapping_add(1);
        let ack_ack = server_seq.wrapping_add(1);

        // Verify cookie
        let valid = verify_cookie(saddr, daddr, sport, dport, ack_seq, ack_ack);
        assert!(valid);

        // Wrong ACK should fail
        let invalid = verify_cookie(saddr, daddr, sport, dport, ack_seq, ack_ack + 1);
        assert!(!invalid);
    }
}
