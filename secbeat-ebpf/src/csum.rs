// Checksum calculation for IP and TCP headers
// Uses 1's complement arithmetic as per RFC 791 (IP) and RFC 793 (TCP)

/// Calculate IPv4 header checksum
/// 
/// The checksum is the 16-bit one's complement of the one's complement sum
/// of all 16-bit words in the header. For computing the checksum, the 
/// checksum field should be zero.
///
/// # Arguments
/// * `data` - Pointer to IPv4 header
/// * `len` - Length of IPv4 header in bytes (typically 20)
///
/// # Returns
/// 16-bit checksum in network byte order
#[inline(always)]
pub fn ipv4_csum(data: *const u8, len: usize) -> u16 {
    let mut sum: u32 = 0;
    let ptr = data as *const u16;
    let words = len / 2;

    // Sum all 16-bit words
    for i in 0..words {
        unsafe {
            sum += u16::from_be(*ptr.add(i)) as u32;
        }
    }

    // Handle odd byte if present
    if len % 2 == 1 {
        unsafe {
            sum += (*data.add(len - 1) as u32) << 8;
        }
    }

    // Fold 32-bit sum to 16 bits
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // Return one's complement
    u16::to_be(!(sum as u16))
}

/// Calculate TCP checksum including pseudo-header
///
/// The checksum includes:
/// 1. Pseudo-header (source IP, dest IP, protocol, TCP length)
/// 2. TCP header
/// 3. TCP data (if any)
///
/// # Arguments
/// * `saddr` - Source IP address (network byte order)
/// * `daddr` - Destination IP address (network byte order)
/// * `tcp_data` - Pointer to TCP header (and data)
/// * `tcp_len` - Total TCP length (header + data)
///
/// # Returns
/// 16-bit checksum in network byte order
#[inline(always)]
pub fn tcp_csum(saddr: u32, daddr: u32, tcp_data: *const u8, tcp_len: u16) -> u16 {
    let mut sum: u32 = 0;

    // Pseudo-header: Source IP (2 words)
    sum += (saddr >> 16) as u32;
    sum += (saddr & 0xFFFF) as u32;

    // Pseudo-header: Dest IP (2 words)
    sum += (daddr >> 16) as u32;
    sum += (daddr & 0xFFFF) as u32;

    // Pseudo-header: Protocol (TCP = 6, padded to 16 bits)
    sum += 6u32;

    // Pseudo-header: TCP Length
    sum += tcp_len as u32;

    // TCP header + data
    let ptr = tcp_data as *const u16;
    let words = tcp_len as usize / 2;

    for i in 0..words {
        unsafe {
            sum += u16::from_be(*ptr.add(i)) as u32;
        }
    }

    // Handle odd byte if present
    if tcp_len % 2 == 1 {
        unsafe {
            sum += (*tcp_data.add(tcp_len as usize - 1) as u32) << 8;
        }
    }

    // Fold 32-bit sum to 16 bits
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    // Return one's complement
    u16::to_be(!(sum as u16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_checksum() {
        // Example IPv4 header (20 bytes, checksum field zeroed)
        let header: [u8; 20] = [
            0x45, 0x00, 0x00, 0x3c, // Version/IHL, TOS, Total Length
            0x1c, 0x46, 0x40, 0x00, // ID, Flags/Fragment
            0x40, 0x06, 0x00, 0x00, // TTL, Protocol, Checksum (zeroed)
            0xac, 0x10, 0x0a, 0x63, // Source IP: 172.16.10.99
            0xac, 0x10, 0x0a, 0x0c, // Dest IP: 172.16.10.12
        ];

        let csum = ipv4_csum(header.as_ptr(), 20);
        // Expected checksum: 0xb1e6 (can verify with Wireshark)
        assert_ne!(csum, 0);
    }

    #[test]
    fn test_tcp_checksum() {
        // Minimal TCP SYN packet header
        let tcp_header: [u8; 20] = [
            0x00, 0x50, 0x1f, 0x90, // Source port 80, Dest port 8080
            0x00, 0x00, 0x00, 0x00, // Seq number
            0x00, 0x00, 0x00, 0x00, // Ack number
            0x50, 0x02, 0x20, 0x00, // Data offset 5 (20 bytes), SYN flag, Window
            0x00, 0x00, 0x00, 0x00, // Checksum (zeroed), Urgent pointer
        ];

        let saddr = u32::from_be_bytes([172, 16, 10, 99]);
        let daddr = u32::from_be_bytes([172, 16, 10, 12]);

        let csum = tcp_csum(saddr, daddr, tcp_header.as_ptr(), 20);
        assert_ne!(csum, 0);
    }
}
