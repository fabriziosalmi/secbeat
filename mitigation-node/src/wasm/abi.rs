// WASM ABI Module - Defines the interface between Host (SecBeat) and Guest (WASM modules)
//
// This module defines the contract for communication between the mitigation node
// and WASM-based WAF rules. The ABI is designed to be simple, fast, and safe.

use serde::{Deserialize, Serialize};

/// Action that a WASM module can return
/// 
/// This represents the decision made by the WASM rule after inspecting a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum Action {
    /// Allow the request to proceed
    Allow = 0,
    
    /// Block the request (drop/reject)
    Block = 1,
    
    /// Log the request but allow it (passive mode)
    Log = 2,
    
    /// Rate limit the request
    RateLimit = 3,
}

impl Action {
    /// Convert from i32 return value to Action
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Action::Allow),
            1 => Some(Action::Block),
            2 => Some(Action::Log),
            3 => Some(Action::RateLimit),
            _ => None,
        }
    }

    /// Convert to i32 for WASM return value
    pub fn to_i32(self) -> i32 {
        self as i32
    }
}

/// Request metadata passed to WASM module
///
/// This is a simplified view of an HTTP request for V1.
/// Future versions can expand this to include headers, body, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    
    /// Request URI/path
    pub uri: String,
    
    /// HTTP version
    pub version: String,
    
    /// Source IP address
    pub source_ip: String,
    
    /// Optional: Headers (can be expensive to pass)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<(String, String)>>,
    
    /// Optional: Request body preview (first N bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
}

impl RequestContext {
    /// Create a minimal request context with just URI
    pub fn minimal(uri: impl Into<String>) -> Self {
        Self {
            method: "GET".to_string(),
            uri: uri.into(),
            version: "HTTP/1.1".to_string(),
            source_ip: "0.0.0.0".to_string(),
            headers: None,
            body_preview: None,
        }
    }

    /// Serialize to JSON for passing to WASM
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON (for WASM module to parse)
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// WASM Module Interface
///
/// Guest (WASM) modules must export this function:
/// ```rust,no_run
/// #[no_mangle]
/// pub extern "C" fn inspect_request(ptr: *const u8, len: usize) -> i32 {
///     // ptr: pointer to JSON-encoded RequestContext
///     // len: length of the JSON string
///     // returns: Action as i32 (0=Allow, 1=Block, etc.)
/// }
/// ```

pub const INSPECT_REQUEST_FN: &str = "inspect_request";

/// Memory allocation function that WASM modules can export
/// (optional - for advanced use cases)
pub const ALLOC_FN: &str = "alloc";

/// Memory deallocation function that WASM modules can export
/// (optional - for advanced use cases)
pub const DEALLOC_FN: &str = "dealloc";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_conversion() {
        assert_eq!(Action::from_i32(0), Some(Action::Allow));
        assert_eq!(Action::from_i32(1), Some(Action::Block));
        assert_eq!(Action::from_i32(2), Some(Action::Log));
        assert_eq!(Action::from_i32(3), Some(Action::RateLimit));
        assert_eq!(Action::from_i32(99), None);

        assert_eq!(Action::Allow.to_i32(), 0);
        assert_eq!(Action::Block.to_i32(), 1);
    }

    #[test]
    fn test_request_context_serialization() {
        let ctx = RequestContext::minimal("/admin/login");
        let json = ctx.to_json().unwrap();
        
        assert!(json.contains("/admin/login"));
        
        let parsed = RequestContext::from_json(&json).unwrap();
        assert_eq!(parsed.uri, "/admin/login");
    }

    #[test]
    fn test_request_context_with_headers() {
        let ctx = RequestContext {
            method: "POST".to_string(),
            uri: "/api/upload".to_string(),
            version: "HTTP/1.1".to_string(),
            source_ip: "192.168.1.100".to_string(),
            headers: Some(vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("User-Agent".to_string(), "BadBot/1.0".to_string()),
            ]),
            body_preview: Some("{\"data\":\"test\"}".to_string()),
        };

        let json = ctx.to_json().unwrap();
        let parsed = RequestContext::from_json(&json).unwrap();
        
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.headers.as_ref().unwrap().len(), 2);
    }
}
