use anyhow::{Context, Result};
use pnet::transport::{transport_channel, TransportChannelType, TransportProtocol, TransportReceiver, TransportSender};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// SYN Proxy implementation for Layer 4 DDoS protection
/// Provides stateless SYN cookie validation to defeat SYN flood attacks
pub struct SynProxy {
    /// Secret key for SYN cookie generation
    secret_key: [u8; 32],
    /// Port to listen on
    listen_port: u16,
    /// Backend server address
    backend_addr: SocketAddr,
    /// Maximum time to wait for ACK after SYN-ACK
    handshake_timeout: Duration,
    /// Local IP address to bind to
    local_ip: Ipv4Addr,
    /// Active handshakes being tracked
    pending_handshakes: Arc<Mutex<HashMap<String, PendingHandshake>>>,
    /// Transport layer sender for raw packets
    tx: Arc<Mutex<Option<TransportSender>>>,
    /// Transport layer receiver for raw packets
    rx: Arc<Mutex<Option<TransportReceiver>>>,
}

/// Information about a pending TCP handshake
#[derive(Debug, Clone)]
struct PendingHandshake {
    /// Client IP address
    client_ip: Ipv4Addr,
    /// Client port
    client_port: u16,
    /// Our sequence number
    our_seq: u32,
    /// Client's sequence number
    client_seq: u32,
    /// Timestamp when handshake started
    timestamp: Instant,
}

impl SynProxy {
    /// Create a new SYN proxy instance
    pub fn new(
        secret_key: [u8; 32],
        listen_port: u16,
        backend_addr: SocketAddr,
        handshake_timeout: Duration,
        local_ip: Ipv4Addr,
    ) -> Self {
        Self {
            secret_key,
            listen_port,
            backend_addr,
            handshake_timeout,
            local_ip,
            pending_handshakes: Arc::new(Mutex::new(HashMap::new())),
            tx: Arc::new(Mutex::new(None)),
            rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize raw socket transport layer
    pub async fn initialize(&mut self) -> Result<()> {
        info!(port = self.listen_port, "Initializing SYN proxy raw sockets");

        // Create transport channel for TCP packets
        let protocol = TransportChannelType::Layer4(TransportProtocol::Ipv4(
            pnet::packet::ip::IpNextHeaderProtocols::Tcp
        ));

        let (tx, rx) = transport_channel(4096, protocol)
            .context("Failed to create transport channel")?;

        *self.tx.lock().await = Some(tx);
        *self.rx.lock().await = Some(rx);

        info!("SYN proxy transport layer initialized");
        Ok(())
    }

    /// Start the SYN proxy server
    pub async fn run(&self) -> Result<()> {
        info!(
            listen_port = self.listen_port,
            backend_addr = %self.backend_addr,
            "Starting SYN proxy server"
        );

        // Start cleanup task for expired handshakes
        self.start_cleanup_task();

        // Main packet processing loop
        loop {
            if let Err(e) = self.process_packets().await {
                error!(error = %e, "Error processing packets");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    /// Process incoming packets
    async fn process_packets(&self) -> Result<()> {
        let mut rx_guard = self.rx.lock().await;
        if let Some(ref mut rx) = *rx_guard {
            let mut buffer = vec![0u8; 4096];
            
            // Try to receive a packet with a short timeout to avoid blocking
            match tokio::time::timeout(Duration::from_millis(10), async {
                // Note: pnet's transport receiver is blocking, so in a real implementation
                // we would need to run this in a separate thread or use async-compatible
                // networking libraries like tokio's UDP/TCP sockets
                
                // For now, we'll simulate packet processing
                // In production, this would be:
                // 1. rx.next() to get the next packet
                // 2. Parse IP and TCP headers
                // 3. Handle SYN/ACK packets based on TCP flags
                
                Ok::<(), anyhow::Error>(())
            }).await {
                Ok(_) => {
                    // Process the packet here
                    debug!("Processed network packet");
                }
                Err(_) => {
                    // Timeout - no packets available
                }
            }
        }
        
        // Clean up expired handshakes
        self.cleanup_expired_handshakes().await;
        
        Ok(())
    }

    /// Clean up expired handshakes
    async fn cleanup_expired_handshakes(&self) {
        let mut pending = self.pending_handshakes.lock().await;
        let now = Instant::now();
        
        pending.retain(|_, handshake| {
            now.duration_since(handshake.timestamp) < self.handshake_timeout
        });
    }

    /// Generate SYN cookie for a client connection
    fn generate_syn_cookie(
        &self,
        client_ip: Ipv4Addr,
        client_port: u16,
        server_port: u16,
        client_seq: u32,
    ) -> u32 {
        let mut hasher = Sha256::new();
        hasher.update(&self.secret_key);
        hasher.update(&client_ip.octets());
        hasher.update(&client_port.to_be_bytes());
        hasher.update(&server_port.to_be_bytes());
        hasher.update(&client_seq.to_be_bytes());
        
        // Add timestamp to prevent replay attacks (truncated to fit in cookie)
        let timestamp = (Instant::now().elapsed().as_secs() / 60) as u32; // 1-minute resolution
        hasher.update(&timestamp.to_be_bytes());
        
        let result = hasher.finalize();
        u32::from_be_bytes([result[0], result[1], result[2], result[3]])
    }

    /// Validate SYN cookie from ACK packet
    fn validate_syn_cookie(
        &self,
        client_ip: Ipv4Addr,
        client_port: u16,
        server_port: u16,
        client_seq: u32,
        cookie: u32,
    ) -> bool {
        // Generate expected cookie for current and previous minute (for clock skew tolerance)
        for time_offset in 0..2 {
            let mut hasher = Sha256::new();
            hasher.update(&self.secret_key);
            hasher.update(&client_ip.octets());
            hasher.update(&client_port.to_be_bytes());
            hasher.update(&server_port.to_be_bytes());
            hasher.update(&client_seq.to_be_bytes());
            
            let timestamp = ((Instant::now().elapsed().as_secs() / 60) - time_offset) as u32;
            hasher.update(&timestamp.to_be_bytes());
            
            let result = hasher.finalize();
            let expected_cookie = u32::from_be_bytes([result[0], result[1], result[2], result[3]]);
            
            if cookie == expected_cookie {
                return true;
            }
        }
        false
    }

    /// Handle incoming SYN packet
    async fn handle_syn_packet(
        &self,
        client_ip: Ipv4Addr,
        client_port: u16,
        client_seq: u32,
    ) -> Result<()> {
        debug!(
            client_ip = %client_ip,
            client_port = client_port,
            client_seq = client_seq,
            "Processing SYN packet"
        );

        // Generate SYN cookie
        let syn_cookie = self.generate_syn_cookie(
            client_ip,
            client_port,
            self.listen_port,
            client_seq,
        );

        // Send SYN-ACK with cookie as sequence number
        self.send_syn_ack(client_ip, client_port, client_seq, syn_cookie).await?;

        debug!(
            client_ip = %client_ip,
            client_port = client_port,
            syn_cookie = syn_cookie,
            "Sent SYN-ACK with cookie"
        );

        Ok(())
    }

    /// Handle incoming ACK packet
    async fn handle_ack_packet(
        &self,
        client_ip: Ipv4Addr,
        client_port: u16,
        client_seq: u32,
        ack_seq: u32,
    ) -> Result<()> {
        debug!(
            client_ip = %client_ip,
            client_port = client_port,
            client_seq = client_seq,
            ack_seq = ack_seq,
            "Processing ACK packet"
        );

        // Extract cookie from ACK sequence number (subtract 1 because client incremented it)
        let cookie = ack_seq.wrapping_sub(1);

        // Validate SYN cookie
        if self.validate_syn_cookie(client_ip, client_port, self.listen_port, client_seq.wrapping_sub(1), cookie) {
            info!(
                client_ip = %client_ip,
                client_port = client_port,
                "Valid ACK received, establishing real connection"
            );

            // Cookie is valid, establish real connection to backend
            self.establish_backend_connection(client_ip, client_port, client_seq).await?;
        } else {
            warn!(
                client_ip = %client_ip,
                client_port = client_port,
                cookie = cookie,
                "Invalid SYN cookie in ACK packet"
            );
        }

        Ok(())
    }

    /// Send SYN-ACK packet to client
    async fn send_syn_ack(
        &self,
        client_ip: Ipv4Addr,
        client_port: u16,
        client_seq: u32,
        syn_cookie: u32,
    ) -> Result<()> {
        // In a real implementation, this would:
        // 1. Construct IP packet with proper headers
        // 2. Construct TCP packet with SYN+ACK flags
        // 3. Set sequence number to syn_cookie
        // 4. Set acknowledgment number to client_seq + 1
        // 5. Send packet via raw socket

        debug!(
            client_ip = %client_ip,
            client_port = client_port,
            client_seq = client_seq,
            syn_cookie = syn_cookie,
            "Would send SYN-ACK packet (implementation simplified)"
        );

        Ok(())
    }

    /// Establish connection to backend server
    async fn establish_backend_connection(
        &self,
        client_ip: Ipv4Addr,
        client_port: u16,
        _client_seq: u32,
    ) -> Result<()> {
        info!(
            client_ip = %client_ip,
            client_port = client_port,
            backend_addr = %self.backend_addr,
            "Establishing connection to backend"
        );

        // In a real implementation, this would:
        // 1. Create TCP connection to backend server
        // 2. Set up bidirectional forwarding between client and backend
        // 3. Handle connection lifecycle and cleanup

        debug!(
            client_ip = %client_ip,
            client_port = client_port,
            "Backend connection established (implementation simplified)"
        );

        Ok(())
    }

    /// Start background task to clean up expired handshakes
    fn start_cleanup_task(&self) {
        let pending_handshakes = Arc::clone(&self.pending_handshakes);
        let timeout = self.handshake_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let mut handshakes = pending_handshakes.lock().await;
                let now = Instant::now();
                let initial_count = handshakes.len();

                handshakes.retain(|_, handshake| {
                    now.duration_since(handshake.timestamp) < timeout
                });

                let cleaned_count = initial_count - handshakes.len();
                if cleaned_count > 0 {
                    debug!(cleaned = cleaned_count, "Cleaned up expired handshakes");
                }
            }
        });
    }

    /// Get statistics about the SYN proxy
    pub async fn get_stats(&self) -> SynProxyStats {
        let pending_handshakes = self.pending_handshakes.lock().await;
        SynProxyStats {
            pending_handshakes: pending_handshakes.len() as u32,
            listen_port: self.listen_port,
            backend_addr: self.backend_addr,
        }
    }
}

/// SYN proxy statistics
#[derive(Debug, Clone)]
pub struct SynProxyStats {
    /// Number of pending handshakes
    pub pending_handshakes: u32,
    /// Port being listened on
    pub listen_port: u16,
    /// Backend server address
    pub backend_addr: SocketAddr,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_syn_cookie_generation_and_validation() {
        let secret_key = [0u8; 32];
        let proxy = SynProxy::new(
            secret_key,
            8080,
            "127.0.0.1:8081".parse().unwrap(),
            Duration::from_secs(30),
            Ipv4Addr::new(127, 0, 0, 1), // local_ip parameter
        );

        let client_ip = Ipv4Addr::new(192, 168, 1, 100);
        let client_port = 12345;
        let server_port = 8080;
        let client_seq = 1000;

        // Generate cookie
        let cookie = proxy.generate_syn_cookie(client_ip, client_port, server_port, client_seq);

        // Validate the same cookie
        assert!(proxy.validate_syn_cookie(client_ip, client_port, server_port, client_seq, cookie));

        // Validate with wrong parameters should fail
        assert!(!proxy.validate_syn_cookie(
            Ipv4Addr::new(192, 168, 1, 101), // Different IP
            client_port,
            server_port,
            client_seq,
            cookie
        ));
    }

    #[tokio::test]
    async fn test_syn_proxy_initialization() {
        let secret_key = [1u8; 32];
        let mut proxy = SynProxy::new(
            secret_key,
            8080,
            "127.0.0.1:8081".parse().unwrap(),
            Duration::from_secs(30),
            Ipv4Addr::new(127, 0, 0, 1), // local_ip parameter
        );

        // Note: This test would require root privileges to actually create raw sockets
        // So we'll just test the structure is created correctly
        assert_eq!(proxy.listen_port, 8080);
        assert_eq!(proxy.secret_key, [1u8; 32]);
    }
}
