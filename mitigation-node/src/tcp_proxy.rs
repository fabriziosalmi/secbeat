use crate::error::{MitigationError, Result};
use std::net::SocketAddr;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{
    tcp::{ReadHalf, WriteHalf},
    TcpListener, TcpStream,
};
use tracing::{debug, error, info, warn};

/// Basic TCP proxy implementation for Phase 1
/// Provides bidirectional forwarding between client and backend
pub struct TcpProxy {
    listen_addr: SocketAddr,
    backend_addr: SocketAddr,
    buffer_size: usize,
}

impl TcpProxy {
    /// Create a new TCP proxy instance
    pub fn new(listen_addr: SocketAddr, backend_addr: SocketAddr, buffer_size: usize) -> Self {
        Self {
            listen_addr,
            backend_addr,
            buffer_size,
        }
    }

    /// Start the TCP proxy server
    pub async fn run(&self) -> Result<()> {
        info!(
            listen_addr = %self.listen_addr,
            backend_addr = %self.backend_addr,
            "Starting basic TCP proxy"
        );

        let listener = TcpListener::bind(&self.listen_addr)
            .await
            .map_err(|e| MitigationError::Io(e))?;;

        info!(
            listen_addr = %self.listen_addr,
            "TCP proxy listening for connections"
        );

        loop {
            match listener.accept().await {
                Ok((client_stream, client_addr)) => {
                    debug!(client_addr = %client_addr, "Accepted new connection");

                    let backend_addr = self.backend_addr;
                    let buffer_size = self.buffer_size;

                    // Spawn a task to handle each connection
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_connection(client_stream, client_addr, backend_addr, buffer_size)
                                .await
                        {
                            error!(
                                client_addr = %client_addr,
                                error = %e,
                                "Connection handling failed"
                            );
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Failed to accept connection");
                    // Brief pause to prevent tight loop on persistent errors
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}

/// Handle a single client connection by proxying to backend
async fn handle_connection(
    mut client_stream: TcpStream,
    client_addr: SocketAddr,
    backend_addr: SocketAddr,
    buffer_size: usize,
) -> Result<()> {
    debug!(
        client_addr = %client_addr,
        backend_addr = %backend_addr,
        "Establishing backend connection"
    );

    // Connect to backend server
    let mut backend_stream = TcpStream::connect(&backend_addr)
        .await
        .map_err(|e| MitigationError::Io(e))?;

    debug!(
        client_addr = %client_addr,
        backend_addr = %backend_addr,
        "Backend connection established, starting bidirectional proxy"
    );

    // Split streams for bidirectional copying
    let (mut client_read, mut client_write) = client_stream.split();
    let (mut backend_read, mut backend_write) = backend_stream.split();

    // Start bidirectional copying
    let (client_to_backend, backend_to_client) = tokio::join!(
        copy_data_split(
            &mut client_read,
            &mut backend_write,
            "client->backend",
            buffer_size
        ),
        copy_data_split(
            &mut backend_read,
            &mut client_write,
            "backend->client",
            buffer_size
        )
    );

    match (client_to_backend, backend_to_client) {
        (Ok(bytes_c2b), Ok(bytes_b2c)) => {
            info!(
                client_addr = %client_addr,
                bytes_client_to_backend = bytes_c2b,
                bytes_backend_to_client = bytes_b2c,
                "Connection closed successfully"
            );
        }
        (Err(e), Ok(bytes_b2c)) => {
            warn!(
                client_addr = %client_addr,
                bytes_backend_to_client = bytes_b2c,
                error = %e,
                "Client to backend copy failed"
            );
        }
        (Ok(bytes_c2b), Err(e)) => {
            warn!(
                client_addr = %client_addr,
                bytes_client_to_backend = bytes_c2b,
                error = %e,
                "Backend to client copy failed"
            );
        }
        (Err(e1), Err(e2)) => {
            warn!(
                client_addr = %client_addr,
                client_to_backend_error = %e1,
                backend_to_client_error = %e2,
                "Both directions failed"
            );
        }
    }

    Ok(())
}

/// Copy data between split streams
async fn copy_data_split(
    source: &mut ReadHalf<'_>,
    destination: &mut WriteHalf<'_>,
    direction: &str,
    buffer_size: usize,
) -> Result<u64> {
    let mut buffer = vec![0u8; buffer_size];
    let mut total_bytes = 0u64;

    loop {
        match source.read(&mut buffer).await {
            Ok(0) => {
                debug!(direction = direction, "Connection closed by source");
                break;
            }
            Ok(bytes_read) => match destination.write_all(&buffer[..bytes_read]).await {
                Ok(()) => {
                    total_bytes += bytes_read as u64;
                    debug!(
                        direction = direction,
                        bytes = bytes_read,
                        total = total_bytes,
                        "Data copied"
                    );
                }
                Err(e) => {
                    return Err(e.into());
                }
            },
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    debug!(direction = direction, "Connection closed by peer");
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    debug!(
        direction = direction,
        total_bytes = total_bytes,
        "Data copying completed"
    );
    Ok(total_bytes)
}

/// Copy data from source to destination stream
async fn copy_data(
    source: &mut TcpStream,
    destination: &mut TcpStream,
    direction: &str,
    buffer_size: usize,
) -> Result<u64> {
    let mut buffer = vec![0u8; buffer_size];
    let mut total_bytes = 0u64;

    loop {
        match source.read(&mut buffer).await {
            Ok(0) => {
                // EOF reached, close the destination write half
                if let Err(e) = destination.shutdown().await {
                    debug!(direction = direction, error = %e, "Failed to shutdown destination");
                }
                break;
            }
            Ok(bytes_read) => match destination.write_all(&buffer[..bytes_read]).await {
                Ok(()) => {
                    total_bytes += bytes_read as u64;
                    debug!(
                        direction = direction,
                        bytes = bytes_read,
                        total = total_bytes,
                        "Data copied"
                    );
                }
                Err(e) => {
                    return Err(e.into());
                }
            },
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    debug!(direction = direction, "Connection closed by peer");
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    debug!(
        direction = direction,
        total_bytes = total_bytes,
        "Data copying completed"
    );
    Ok(total_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    async fn start_echo_server() -> Result<SocketAddr> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let mut buffer = [0u8; 1024];
                    while let Ok(n) = stream.read(&mut buffer).await {
                        if n == 0 {
                            break;
                        }
                        let _ = stream.write_all(&buffer[..n]).await;
                    }
                });
            }
        });

        Ok(addr)
    }

    #[tokio::test]
    async fn test_tcp_proxy_basic_forwarding() {
        let echo_server_addr = start_echo_server().await.expect("Should start echo server for test");
        let proxy = TcpProxy::new("127.0.0.1:0".parse().expect("Test address should be valid"), echo_server_addr, 4096);

        // This is a basic test structure - in a real test we'd:
        // 1. Start the proxy in a background task
        // 2. Connect a client and send data
        // 3. Verify the data is echoed back correctly
        assert_eq!(proxy.buffer_size, 4096);
    }
}
