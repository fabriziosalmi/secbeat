use thiserror::Error;

/// Errors that can occur in the mitigation node library
#[derive(Error, Debug)]
pub enum MitigationError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Secret management error
    #[error("Secret error: {0}")]
    Secret(String),

    /// Network I/O error
    #[error("Network I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// WAF error
    #[error("WAF error: {0}")]
    Waf(String),

    /// WASM runtime error
    #[error("WASM runtime error: {0}")]
    Wasm(String),

    /// DDoS protection error
    #[error("DDoS protection error: {0}")]
    Ddos(String),

    /// BPF/eBPF error (Linux only)
    #[cfg(target_os = "linux")]
    #[error("BPF error: {0}")]
    Bpf(String),

    /// Orchestrator communication error
    #[error("Orchestrator error: {0}")]
    Orchestrator(String),

    /// Invalid state error
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Resource exhaustion error
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

/// Result type alias using MitigationError
pub type Result<T> = std::result::Result<T, MitigationError>;

impl From<String> for MitigationError {
    fn from(s: String) -> Self {
        MitigationError::Other(s)
    }
}

impl From<&str> for MitigationError {
    fn from(s: &str) -> Self {
        MitigationError::Other(s.to_string())
    }
}

impl From<serde_json::Error> for MitigationError {
    fn from(err: serde_json::Error) -> Self {
        MitigationError::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for MitigationError {
    fn from(err: serde_yaml::Error) -> Self {
        MitigationError::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MitigationError::Config("invalid port".to_string());
        assert_eq!(err.to_string(), "Configuration error: invalid port");
    }

    #[test]
    fn test_error_from_string() {
        let err: MitigationError = "test error".into();
        assert!(matches!(err, MitigationError::Other(_)));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: MitigationError = io_err.into();
        assert!(matches!(err, MitigationError::Io(_)));
    }
}
