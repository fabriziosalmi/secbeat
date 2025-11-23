// Universal WAF - Data-Driven WASM Rule Engine
//
// This WASM module executes dynamic rules configured at runtime.
// Instead of hardcoded logic, it reads a JSON configuration with rules
// and applies them to incoming HTTP requests.
//
// Configuration Schema:
// {
//   "rules": [
//     {
//       "id": "rule-001",
//       "field": "Header:User-Agent",
//       "pattern": "EvilBot.*",
//       "action": "Block"
//     },
//     {
//       "field": "URI",
//       "pattern": "^/admin",
//       "action": "Block"
//     }
//   ]
// }

use serde::{Deserialize, Serialize};
use std::cell::RefCell;

// ============================================================================
// Configuration Schema
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafConfig {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub field: String,      // "URI", "Method", "Header:X", "SourceIP"
    pub pattern: String,    // Regex pattern or exact match
    pub action: String,     // "Block", "Allow", "Log", "RateLimit"
}

// ============================================================================
// Request Context (from host)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub source_ip: String,
    #[serde(default)]
    pub headers: Option<Vec<(String, String)>>,
    #[serde(default)]
    pub body_preview: Option<String>,
}

// ============================================================================
// Action Result
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Action {
    Allow = 0,
    Block = 1,
    Log = 2,
    RateLimit = 3,
}

// ============================================================================
// Global Configuration Storage
// ============================================================================

// WASM doesn't have traditional multithreading, so static mut is safe here
static mut CONFIG: RefCell<Option<WafConfig>> = RefCell::new(None);

// ============================================================================
// Exported Functions
// ============================================================================

/// Configure the WAF with a JSON config
/// Called by host immediately after module instantiation
#[no_mangle]
pub extern "C" fn configure(ptr: *const u8, len: usize) -> i32 {
    // Read JSON from memory
    let config_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    
    let config_str = match std::str::from_utf8(config_bytes) {
        Ok(s) => s,
        Err(_) => return -1, // Invalid UTF-8
    };

    // Parse config
    let config: WafConfig = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(_) => return -2, // Invalid JSON
    };

    // Store config globally
    unsafe {
        *CONFIG.borrow_mut() = Some(config);
    }

    0 // Success
}

/// Inspect incoming HTTP request
/// Returns Action as i32
#[no_mangle]
pub extern "C" fn inspect_request(ptr: *const u8, len: usize) -> i32 {
    // Parse request context
    let json_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    
    let json_str = match std::str::from_utf8(json_bytes) {
        Ok(s) => s,
        Err(_) => return Action::Allow as i32, // Fail open
    };

    let ctx: RequestContext = match serde_json::from_str(json_str) {
        Ok(c) => c,
        Err(_) => return Action::Allow as i32, // Fail open
    };

    // Get config
    let config = unsafe {
        match &*CONFIG.borrow() {
            Some(c) => c.clone(),
            None => return Action::Allow as i32, // No config = allow all
        }
    };

    // Apply rules in order (first match wins)
    for rule in &config.rules {
        if rule_matches(&rule, &ctx) {
            return action_from_string(&rule.action) as i32;
        }
    }

    // Default: Allow
    Action::Allow as i32
}

// ============================================================================
// Rule Matching Logic
// ============================================================================

fn rule_matches(rule: &Rule, ctx: &RequestContext) -> bool {
    let field_value = extract_field(&rule.field, ctx);
    
    match field_value {
        Some(value) => pattern_matches(&rule.pattern, &value),
        None => false,
    }
}

fn extract_field(field: &str, ctx: &RequestContext) -> Option<String> {
    match field {
        "URI" => Some(ctx.uri.clone()),
        "Method" => Some(ctx.method.clone()),
        "SourceIP" => Some(ctx.source_ip.clone()),
        "Version" => Some(ctx.version.clone()),
        _ if field.starts_with("Header:") => {
            let header_name = &field[7..]; // Strip "Header:" prefix
            ctx.headers.as_ref().and_then(|headers| {
                headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(header_name))
                    .map(|(_, value)| value.clone())
            })
        }
        "Body" => ctx.body_preview.clone(),
        _ => None,
    }
}

fn pattern_matches(pattern: &str, value: &str) -> bool {
    // Simple pattern matching (exact match or contains)
    // For production, use regex crate (but increases binary size)
    
    if pattern.starts_with('^') && pattern.ends_with('$') {
        // Exact match
        let clean_pattern = &pattern[1..pattern.len()-1];
        value == clean_pattern
    } else if pattern.starts_with('^') {
        // Starts with
        let clean_pattern = &pattern[1..];
        value.starts_with(clean_pattern)
    } else if pattern.ends_with('$') {
        // Ends with
        let clean_pattern = &pattern[..pattern.len()-1];
        value.ends_with(clean_pattern)
    } else if pattern.contains('*') {
        // Wildcard matching (simple glob)
        simple_glob_match(pattern, value)
    } else {
        // Contains
        value.to_lowercase().contains(&pattern.to_lowercase())
    }
}

fn simple_glob_match(pattern: &str, value: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    
    if parts.is_empty() {
        return true;
    }

    let value_lower = value.to_lowercase();
    let mut current_pos = 0;

    for (i, part) in parts.iter().enumerate() {
        let part_lower = part.to_lowercase();
        
        if part_lower.is_empty() {
            continue;
        }

        if i == 0 {
            // First part must match at start
            if !value_lower.starts_with(&part_lower) {
                return false;
            }
            current_pos = part_lower.len();
        } else if i == parts.len() - 1 {
            // Last part must match at end
            if !value_lower.ends_with(&part_lower) {
                return false;
            }
        } else {
            // Middle parts must exist somewhere after current position
            match value_lower[current_pos..].find(&part_lower) {
                Some(pos) => {
                    current_pos += pos + part_lower.len();
                }
                None => return false,
            }
        }
    }

    true
}

fn action_from_string(action: &str) -> Action {
    match action.to_lowercase().as_str() {
        "block" => Action::Block,
        "log" => Action::Log,
        "ratelimit" => Action::RateLimit,
        _ => Action::Allow,
    }
}

// ============================================================================
// Tests (compile with --test, not for WASM target)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("admin", "/admin/login"));
        assert!(pattern_matches("^/api", "/api/users"));
        assert!(!pattern_matches("^/api", "/login/api"));
        assert!(pattern_matches("/login$", "/admin/login"));
        assert!(pattern_matches("EvilBot*", "EvilBot/1.0"));
        assert!(pattern_matches("*sqlmap*", "Mozilla/5.0 sqlmap/1.0"));
    }

    #[test]
    fn test_glob_matching() {
        assert!(simple_glob_match("Evil*", "EvilBot"));
        assert!(simple_glob_match("*Bot*", "EvilBot/1.0"));
        assert!(simple_glob_match("Mozilla*sqlmap*", "Mozilla/5.0 sqlmap"));
        assert!(!simple_glob_match("Evil*", "GoodBot"));
    }

    #[test]
    fn test_field_extraction() {
        let ctx = RequestContext {
            method: "GET".to_string(),
            uri: "/admin".to_string(),
            version: "HTTP/1.1".to_string(),
            source_ip: "1.2.3.4".to_string(),
            headers: Some(vec![
                ("User-Agent".to_string(), "BadBot/1.0".to_string()),
            ]),
            body_preview: None,
        };

        assert_eq!(extract_field("URI", &ctx), Some("/admin".to_string()));
        assert_eq!(extract_field("Method", &ctx), Some("GET".to_string()));
        assert_eq!(
            extract_field("Header:User-Agent", &ctx),
            Some("BadBot/1.0".to_string())
        );
    }
}
