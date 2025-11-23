// Bad Bot Detection WASM Rule
//
// This is a sample WASM module that demonstrates the SecBeat WASM ABI.
// It blocks requests to /admin paths and logs suspicious user agents.

use serde::{Deserialize, Serialize};

/// Action enum (must match host ABI)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Action {
    Allow = 0,
    Block = 1,
    Log = 2,
    RateLimit = 3,
}

/// Request context (must match host ABI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub source_ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
}

/// Main entry point - called by the host with request context
///
/// # Arguments
/// * `ptr` - Pointer to JSON-encoded RequestContext in WASM memory
/// * `len` - Length of the JSON string
///
/// # Returns
/// Action as i32 (0=Allow, 1=Block, 2=Log, 3=RateLimit)
#[no_mangle]
pub extern "C" fn inspect_request(ptr: *const u8, len: usize) -> i32 {
    // Safety: Host guarantees ptr is valid for len bytes
    let json_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    
    // Parse JSON
    let json_str = match std::str::from_utf8(json_bytes) {
        Ok(s) => s,
        Err(_) => return Action::Block as i32, // Invalid UTF-8 = suspicious
    };

    let ctx: RequestContext = match serde_json::from_str(json_str) {
        Ok(ctx) => ctx,
        Err(_) => return Action::Block as i32, // Invalid JSON = suspicious
    };

    // Rule 1: Block /admin paths
    if ctx.uri.contains("/admin") {
        return Action::Block as i32;
    }

    // Rule 2: Block known bad bot user agents
    if let Some(ref headers) = ctx.headers {
        for (name, value) in headers {
            if name.to_lowercase() == "user-agent" {
                if is_bad_bot(value) {
                    return Action::Block as i32;
                }
            }
        }
    }

    // Rule 3: Log suspicious patterns (but allow)
    if is_suspicious(&ctx.uri) {
        return Action::Log as i32;
    }

    // Default: Allow
    Action::Allow as i32
}

/// Check if user agent matches known bad bots
fn is_bad_bot(user_agent: &str) -> bool {
    let ua_lower = user_agent.to_lowercase();
    
    // Known malicious bots
    let bad_patterns = [
        "badbot",
        "sqlmap",
        "nikto",
        "masscan",
        "zgrab",
        "nmap",
        "metasploit",
    ];

    bad_patterns.iter().any(|pattern| ua_lower.contains(pattern))
}

/// Check if URI contains suspicious patterns
fn is_suspicious(uri: &str) -> bool {
    let uri_lower = uri.to_lowercase();
    
    // SQL injection attempts
    if uri_lower.contains("' or ") || uri_lower.contains("union select") {
        return true;
    }

    // Path traversal attempts
    if uri_lower.contains("../") || uri_lower.contains("..\\") {
        return true;
    }

    // Command injection attempts
    if uri_lower.contains(";") && (uri_lower.contains("wget") || uri_lower.contains("curl")) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bad_bot() {
        assert!(is_bad_bot("BadBot/1.0"));
        assert!(is_bad_bot("Mozilla/5.0 (compatible; sqlmap/1.0)"));
        assert!(!is_bad_bot("Mozilla/5.0 (Windows NT 10.0)"));
    }

    #[test]
    fn test_is_suspicious() {
        assert!(is_suspicious("/api/users?id=1' or 1=1--"));
        assert!(is_suspicious("/files/../../etc/passwd"));
        assert!(!is_suspicious("/api/users?id=123"));
    }
}
