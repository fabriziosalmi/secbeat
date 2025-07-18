use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

/// Simple HTTP origin server for testing the proxy
pub struct TestOriginServer {
    listen_addr: SocketAddr,
}

impl TestOriginServer {
    pub fn new(listen_addr: SocketAddr) -> Self {
        Self { listen_addr }
    }

    /// Start the test origin server
    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.listen_addr).await?;
        info!(
            listen_addr = %self.listen_addr,
            "Test origin server started"
        );

        loop {
            match listener.accept().await {
                Ok((stream, client_addr)) => {
                    info!(client_addr = %client_addr, "Origin: New connection");
                    tokio::spawn(async move {
                        if let Err(e) = handle_http_request(stream, client_addr).await {
                            error!(
                                client_addr = %client_addr,
                                error = %e,
                                "Origin: Failed to handle request"
                            );
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "Origin: Failed to accept connection");
                }
            }
        }
    }
}

/// Handle a single HTTP request with a simple response
async fn handle_http_request(mut stream: TcpStream, client_addr: SocketAddr) -> Result<()> {
    let mut buffer = vec![0u8; 4096];

    // Read the HTTP request
    match stream.read(&mut buffer).await {
        Ok(0) => {
            warn!(client_addr = %client_addr, "Origin: Connection closed by client");
            return Ok(());
        }
        Ok(bytes_read) => {
            let request = String::from_utf8_lossy(&buffer[..bytes_read]);
            info!(
                client_addr = %client_addr,
                bytes_read = bytes_read,
                "Origin: Received HTTP request"
            );

            // Log first line of HTTP request
            if let Some(first_line) = request.lines().next() {
                info!(client_addr = %client_addr, request_line = %first_line, "Origin: HTTP request line");
            }
        }
        Err(e) => {
            error!(client_addr = %client_addr, error = %e, "Origin: Failed to read request");
            return Err(e.into());
        }
    }

    // Send a simple HTTP response
    let response = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: text/html\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Server: TestOrigin/1.0\r\n\
         \r\n\
         {}",
        get_test_html().len(),
        get_test_html()
    );

    match stream.write_all(response.as_bytes()).await {
        Ok(()) => {
            info!(
                client_addr = %client_addr,
                response_bytes = response.len(),
                "Origin: Sent HTTP response"
            );
        }
        Err(e) => {
            error!(client_addr = %client_addr, error = %e, "Origin: Failed to send response");
            return Err(e.into());
        }
    }

    // Gracefully close the connection
    if let Err(e) = stream.shutdown().await {
        warn!(client_addr = %client_addr, error = %e, "Origin: Failed to shutdown connection");
    }

    Ok(())
}

/// Generate test HTML content
fn get_test_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>Test Origin Server - SecBeat Mitigation Testing</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 20px; border-radius: 8px; }
        .header { color: #2c3e50; border-bottom: 2px solid #3498db; padding-bottom: 10px; }
        .status { background: #27ae60; color: white; padding: 10px; border-radius: 4px; margin: 20px 0; }
        .metrics { background: #ecf0f1; padding: 15px; border-radius: 4px; }
    </style>
</head>
<body>
    <div class="container">
        <h1 class="header">🛡️ SecBeat Test Origin Server</h1>
        <div class="status">✅ Status: Online and Ready</div>
        <div class="metrics">
            <h3>Server Information:</h3>
            <ul>
                <li><strong>Server:</strong> TestOrigin/1.0</li>
                <li><strong>Network:</strong> 192.168.100.x</li>
                <li><strong>Purpose:</strong> DDoS Mitigation Testing</li>
                <li><strong>Timestamp:</strong> <span id="timestamp"></span></li>
            </ul>
        </div>
        <h3>🔧 Testing Proxy Functionality</h3>
        <p>This origin server is designed to test the SecBeat mitigation node proxy capabilities.</p>
        <p>If you're seeing this page, the TCP proxy is working correctly!</p>
    </div>
    <script>
        document.getElementById('timestamp').textContent = new Date().toISOString();
    </script>
</body>
</html>"#
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "test_origin=info".into()),
        )
        .with_target(false)
        .with_thread_ids(true)
        .init();

    // Start test origin server on 127.0.0.1:8080
    let origin_addr: SocketAddr = "127.0.0.1:8080".parse()?;
    let server = TestOriginServer::new(origin_addr);

    info!("🚀 Starting Test Origin Server for SecBeat Mitigation Testing");
    server.run().await
}
