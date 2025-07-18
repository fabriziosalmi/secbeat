use crate::config::WafConfig;
use anyhow::{Context, Result};
use metrics::counter;
use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Web Application Firewall engine
#[derive(Debug, Clone)]
pub struct WafEngine {
    config: WafConfig,
    /// Compiled attack pattern regexes
    sql_injection_patterns: Vec<Regex>,
    xss_patterns: Vec<Regex>,
    path_traversal_patterns: Vec<Regex>,
    command_injection_patterns: Vec<Regex>,
    custom_patterns: Vec<Regex>,
}

/// Result of WAF inspection
#[derive(Debug, Clone, PartialEq)]
pub enum WafResult {
    /// Allow the request
    Allow,
    /// Block due to SQL injection attempt
    SqlInjection,
    /// Block due to XSS attempt
    XssAttempt,
    /// Block due to path traversal attempt
    PathTraversal,
    /// Block due to command injection attempt
    CommandInjection,
    /// Block due to custom pattern match
    CustomPattern(String),
    /// Block due to invalid HTTP
    InvalidHttp,
    /// Block due to oversized request
    OversizedRequest,
}

/// Parsed HTTP request for inspection
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub query_string: Option<String>,
}

impl WafEngine {
    /// Create new WAF engine
    pub async fn new(config: WafConfig) -> Result<Self> {
        info!("Initializing WAF engine");

        let mut waf = Self {
            config,
            sql_injection_patterns: Vec::new(),
            xss_patterns: Vec::new(),
            path_traversal_patterns: Vec::new(),
            command_injection_patterns: Vec::new(),
            custom_patterns: Vec::new(),
        };

        waf.compile_patterns().await?;

        info!(
            sql_patterns = waf.sql_injection_patterns.len(),
            xss_patterns = waf.xss_patterns.len(),
            path_traversal_patterns = waf.path_traversal_patterns.len(),
            command_patterns = waf.command_injection_patterns.len(),
            custom_patterns = waf.custom_patterns.len(),
            "WAF engine initialized"
        );

        Ok(waf)
    }

    /// Compile attack pattern regexes
    async fn compile_patterns(&mut self) -> Result<()> {
        // SQL Injection patterns
        if self.config.attack_patterns.sql_injection {
            let sql_patterns = vec![
                "(?i)(union\\s+select)",
                "(?i)(select\\s+.*\\s+from)",
                "(?i)(insert\\s+into)",
                "(?i)(delete\\s+from)",
                "(?i)(drop\\s+table)",
                "(?i)('\\s*or\\s+\\d+\\s*=\\s*\\d+)",
                "(?i)(or\\s+1\\s*=\\s*1)",
                "(?i)(and\\s+1\\s*=\\s*1)",
                "(?i)(exec\\s*\\()",
                "(?i)(sp_executesql)",
                "(?i)(xp_cmdshell)",
                "(?i)(benchmark\\s*\\()",
                "(?i)(sleep\\s*\\()",
                "(?i)(waitfor\\s+delay)",
            ];

            for pattern in sql_patterns {
                match Regex::new(pattern) {
                    Ok(regex) => self.sql_injection_patterns.push(regex),
                    Err(e) => {
                        warn!(pattern = pattern, error = %e, "Failed to compile SQL injection pattern")
                    }
                }
            }
        }

        // XSS patterns
        if self.config.attack_patterns.xss_detection {
            let xss_patterns = vec![
                "(?i)<script[^>]*>",
                "(?i)</script>",
                "(?i)<iframe[^>]*>",
                "(?i)<object[^>]*>",
                "(?i)<embed[^>]*>",
                "(?i)<form[^>]*>",
                "(?i)javascript:",
                "(?i)vbscript:",
                "(?i)onload\\s*=",
                "(?i)onerror\\s*=",
                "(?i)onclick\\s*=",
                "(?i)onmouseover\\s*=",
                "(?i)onfocus\\s*=",
                "(?i)onblur\\s*=",
                "(?i)onchange\\s*=",
                "(?i)onsubmit\\s*=",
                "(?i)expression\\s*\\(",
                "(?i)url\\s*\\(",
                "(?i)@import",
                "(?i)<img[^>]*src\\s*=\\s*['\"]?javascript:",
            ];

            for pattern in xss_patterns {
                match Regex::new(pattern) {
                    Ok(regex) => self.xss_patterns.push(regex),
                    Err(e) => warn!(pattern = pattern, error = %e, "Failed to compile XSS pattern"),
                }
            }
        }

        // Path traversal patterns
        if self.config.attack_patterns.path_traversal {
            let path_patterns = vec![
                "\\.\\./",
                "\\.\\.\\\\/",
                "(?i)/etc/passwd",
                "(?i)/etc/shadow",
                "(?i)/etc/hosts",
                "(?i)/proc/",
                "(?i)/sys/",
                "(?i)c:\\\\windows",
            ];

            for pattern in path_patterns {
                match Regex::new(pattern) {
                    Ok(regex) => self.path_traversal_patterns.push(regex),
                    Err(e) => {
                        warn!(pattern = pattern, error = %e, "Failed to compile path traversal pattern")
                    }
                }
            }
        }

        // Command injection patterns
        if self.config.attack_patterns.command_injection {
            let cmd_patterns = vec![
                "(?i)(;|\\||&&)\\s*(cat|ls|dir|type|more|less)",
                "(?i)(;|\\||&&)\\s*(wget|curl|nc|netcat)",
                "(?i)(;|\\||&&)\\s*(rm|del|rmdir|rd)",
                "(?i)(;|\\||&&)\\s*(ps|tasklist|netstat)",
                "(?i)(;|\\||&&)\\s*(whoami|id|pwd)",
                "(?i)(;|\\||&&)\\s*(chmod|chown|chgrp)",
                "(?i)/bin/(sh|bash|csh|tcsh|zsh)",
                "(?i)cmd\\.exe",
                "(?i)powershell",
            ];

            for pattern in cmd_patterns {
                match Regex::new(pattern) {
                    Ok(regex) => self.command_injection_patterns.push(regex),
                    Err(e) => {
                        warn!(pattern = pattern, error = %e, "Failed to compile command injection pattern")
                    }
                }
            }
        }

        // Custom patterns - load from file if configured
        if self
            .config
            .attack_patterns
            .custom_rules_enabled
            .unwrap_or(false)
        {
            if let Some(ref custom_rules_path) = self.config.attack_patterns.custom_rules_path {
                match self.load_custom_patterns(custom_rules_path).await {
                    Ok(patterns) => {
                        self.custom_patterns = patterns;
                        info!(
                            path = %custom_rules_path,
                            count = self.custom_patterns.len(),
                            "Loaded custom WAF patterns"
                        );
                    }
                    Err(e) => {
                        warn!(
                            path = %custom_rules_path,
                            error = %e,
                            "Failed to load custom WAF patterns, continuing without them"
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Load custom patterns from file
    async fn load_custom_patterns(&self, file_path: &str) -> Result<Vec<Regex>> {
        use serde::Deserialize;
        use tokio::fs;

        #[derive(Deserialize)]
        struct CustomRuleFile {
            patterns: Vec<String>,
        }

        let content = fs::read_to_string(file_path)
            .await
            .context("Failed to read custom rules file")?;

        let rules: CustomRuleFile = serde_yaml::from_str(&content)
            .or_else(|_| serde_json::from_str(&content))
            .context("Failed to parse custom rules file (expected JSON or YAML)")?;

        let mut compiled_patterns = Vec::new();
        for pattern in rules.patterns {
            match Regex::new(&pattern) {
                Ok(regex) => {
                    compiled_patterns.push(regex);
                    debug!(pattern = %pattern, "Compiled custom WAF pattern");
                }
                Err(e) => {
                    warn!(pattern = %pattern, error = %e, "Failed to compile custom WAF pattern");
                }
            }
        }

        Ok(compiled_patterns)
    }

    /// Reload patterns from configuration (for runtime updates)
    pub async fn reload_patterns(&mut self) -> Result<()> {
        // Clear existing patterns
        self.sql_injection_patterns.clear();
        self.xss_patterns.clear();
        self.path_traversal_patterns.clear();
        self.command_injection_patterns.clear();
        self.custom_patterns.clear();

        // Reinitialize patterns
        self.compile_patterns().await
    }

    /// Add a custom pattern at runtime
    pub async fn add_custom_pattern(&mut self, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern).context("Failed to compile custom pattern")?;

        self.custom_patterns.push(regex);
        info!(pattern = %pattern, "Added custom WAF pattern");
        Ok(())
    }

    /// Remove custom patterns matching a specific pattern
    pub async fn remove_custom_pattern(&mut self, pattern: &str) -> Result<usize> {
        let initial_count = self.custom_patterns.len();
        self.custom_patterns
            .retain(|regex| regex.as_str() != pattern);
        let removed_count = initial_count - self.custom_patterns.len();

        if removed_count > 0 {
            info!(pattern = %pattern, count = removed_count, "Removed custom WAF patterns");
        }

        Ok(removed_count)
    }

    /// Inspect HTTP request
    pub fn inspect_request(&self, request: &HttpRequest) -> WafResult {
        if !self.config.enabled {
            return WafResult::Allow;
        }

        debug!(
            method = %request.method,
            path = %request.path,
            "Inspecting HTTP request"
        );

        // Check request size
        if let Some(body) = &request.body {
            if body.len()
                > self
                    .config
                    .http_inspection
                    .max_body_size_bytes
                    .unwrap_or(1024 * 1024)
            {
                debug!(
                    body_size = body.len(),
                    max_size = self
                        .config
                        .http_inspection
                        .max_body_size_bytes
                        .unwrap_or(1024 * 1024),
                    "Request body too large"
                );
                counter!("waf_blocked_oversized", 1);
                return WafResult::OversizedRequest;
            }
        }

        // Inspect URL
        if self.config.http_inspection.inspect_url {
            if let Some(result) = self.inspect_string(&request.path, "url") {
                return result;
            }

            if let Some(query) = &request.query_string {
                if let Some(result) = self.inspect_string(query, "query") {
                    return result;
                }
            }
        }

        // Inspect headers
        if self.config.http_inspection.inspect_headers {
            for (name, value) in &request.headers {
                if let Some(result) = self.inspect_string(value, &format!("header:{name}")) {
                    return result;
                }
            }
        }

        // Inspect body
        if self.config.http_inspection.inspect_body {
            if let Some(body) = &request.body {
                if let Ok(body_str) = std::str::from_utf8(body) {
                    if let Some(result) = self.inspect_string(body_str, "body") {
                        return result;
                    }
                }
            }
        }

        WafResult::Allow
    }

    /// Inspect a string for attack patterns
    fn inspect_string(&self, content: &str, location: &str) -> Option<WafResult> {
        // SQL Injection
        for pattern in &self.sql_injection_patterns {
            if pattern.is_match(content) {
                debug!(
                    location = location,
                    pattern = pattern.as_str(),
                    content = %content,
                    "SQL injection pattern matched"
                );
                counter!("waf_blocked_sql_injection", 1);
                return Some(WafResult::SqlInjection);
            }
        }

        // XSS
        for pattern in &self.xss_patterns {
            if pattern.is_match(content) {
                debug!(
                    location = location,
                    pattern = pattern.as_str(),
                    content = %content,
                    "XSS pattern matched"
                );
                counter!("waf_blocked_xss", 1);
                return Some(WafResult::XssAttempt);
            }
        }

        // Path Traversal
        for pattern in &self.path_traversal_patterns {
            if pattern.is_match(content) {
                debug!(
                    location = location,
                    pattern = pattern.as_str(),
                    content = %content,
                    "Path traversal pattern matched"
                );
                counter!("waf_blocked_path_traversal", 1);
                return Some(WafResult::PathTraversal);
            }
        }

        // Command Injection
        for pattern in &self.command_injection_patterns {
            if pattern.is_match(content) {
                debug!(
                    location = location,
                    pattern = pattern.as_str(),
                    content = %content,
                    "Command injection pattern matched"
                );
                counter!("waf_blocked_command_injection", 1);
                return Some(WafResult::CommandInjection);
            }
        }

        // Custom patterns
        for (i, pattern) in self.custom_patterns.iter().enumerate() {
            if pattern.is_match(content) {
                debug!(
                    location = location,
                    pattern = pattern.as_str(),
                    pattern_index = i,
                    content = %content,
                    "Custom pattern matched"
                );
                counter!("waf_blocked_custom_pattern", 1);
                return Some(WafResult::CustomPattern(format!("pattern_{i}")));
            }
        }

        None
    }

    /// Parse HTTP request from raw bytes
    pub fn parse_http_request(&self, data: &[u8]) -> Result<HttpRequest> {
        let request_str = std::str::from_utf8(data)?;
        let mut lines = request_str.lines();

        // Parse request line
        let request_line = lines
            .next()
            .ok_or_else(|| anyhow::anyhow!("Empty request"))?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();

        if parts.len() != 3 {
            return Err(anyhow::anyhow!("Invalid request line"));
        }

        let method = parts[0].to_string();
        let path_and_query = parts[1];
        let version = parts[2].to_string();

        // Split path and query string
        let (path, query_string) = if let Some(pos) = path_and_query.find('?') {
            let path = path_and_query[..pos].to_string();
            let query = path_and_query[pos + 1..].to_string();
            (path, Some(query))
        } else {
            (path_and_query.to_string(), None)
        };

        // Parse headers
        let mut headers = HashMap::new();
        let mut body_start = 0;

        for (i, line) in lines.enumerate() {
            if line.is_empty() {
                body_start = i + 1;
                break;
            }

            if let Some(pos) = line.find(':') {
                let name = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().to_string();
                headers.insert(name, value);
            }
        }

        // Extract body if present
        let body = if body_start > 0 {
            let header_end = request_str
                .lines()
                .take(body_start)
                .map(|l| l.len() + 1)
                .sum::<usize>();
            if header_end < data.len() {
                Some(data[header_end..].to_vec())
            } else {
                None
            }
        } else {
            None
        };

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body,
            query_string,
        })
    }

    /// Get WAF statistics
    pub async fn get_stats(&self) -> WafStats {
        WafStats {
            enabled: self.config.enabled,
            sql_patterns: self.sql_injection_patterns.len() as u32,
            xss_patterns: self.xss_patterns.len() as u32,
            path_traversal_patterns: self.path_traversal_patterns.len() as u32,
            command_injection_patterns: self.command_injection_patterns.len() as u32,
            custom_patterns: self.custom_patterns.len() as u32,
        }
    }
}

/// WAF statistics
#[derive(Debug, Clone)]
pub struct WafStats {
    pub enabled: bool,
    pub sql_patterns: u32,
    pub xss_patterns: u32,
    pub path_traversal_patterns: u32,
    pub command_injection_patterns: u32,
    pub custom_patterns: u32,
}
