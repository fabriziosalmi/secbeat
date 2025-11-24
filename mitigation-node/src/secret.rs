use crate::error::{MitigationError, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// A secret string that prevents accidental logging
/// 
/// This wrapper ensures secrets are not printed in logs or Debug output.
/// The actual value is only accessible through explicit methods.
#[derive(Clone)]
pub struct Secret<T> {
    inner: T,
}

impl<T> Secret<T> {
    /// Create a new secret from a value
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Expose the secret value (use with caution)
    pub fn expose_secret(&self) -> &T {
        &self.inner
    }

    /// Consume the secret and return the inner value
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl Secret<String> {
    /// Create a secret from an environment variable
    /// 
    /// # Errors
    /// Returns an error if the environment variable is not set or is empty
    pub fn from_env(var_name: &str) -> Result<Self> {
        std::env::var(var_name)
            .map_err(|e| MitigationError::Secret(format!("Environment variable '{}' not set: {}", var_name, e)))
            .and_then(|val| {
                if val.is_empty() {
                    Err(MitigationError::Secret(format!("Environment variable '{}' is empty", var_name)))
                } else {
                    Ok(Self::new(val))
                }
            })
    }

    /// Create a secret from an optional environment variable with a default
    pub fn from_env_or(var_name: &str, default: String) -> Self {
        Self::from_env(var_name).unwrap_or_else(|_| Self::new(default))
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T: Clone> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

// Serde support for Secret<String>
impl Serialize for Secret<String> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Never serialize the actual value
        serializer.serialize_str("[REDACTED]")
    }
}

impl<'de> Deserialize<'de> for Secret<String> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the string value and wrap it
        let value = String::deserialize(deserializer)?;
        Ok(Secret::new(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_debug() {
        let secret = Secret::new("my-secret-password".to_string());
        let debug_output = format!("{:?}", secret);
        assert_eq!(debug_output, "[REDACTED]");
        assert!(!debug_output.contains("password"));
    }

    #[test]
    fn test_secret_display() {
        let secret = Secret::new("my-secret-password".to_string());
        let display_output = format!("{}", secret);
        assert_eq!(display_output, "[REDACTED]");
        assert!(!display_output.contains("password"));
    }

    #[test]
    fn test_secret_expose() {
        let secret = Secret::new("my-secret-password".to_string());
        assert_eq!(secret.expose_secret(), "my-secret-password");
    }

    #[test]
    fn test_secret_from_env() {
        std::env::set_var("TEST_SECRET", "test-value");
        let secret = Secret::<String>::from_env("TEST_SECRET").unwrap();
        assert_eq!(secret.expose_secret(), "test-value");
        std::env::remove_var("TEST_SECRET");
    }

    #[test]
    fn test_secret_from_env_empty() {
        std::env::set_var("TEST_SECRET_EMPTY", "");
        let result = Secret::<String>::from_env("TEST_SECRET_EMPTY");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
        std::env::remove_var("TEST_SECRET_EMPTY");
    }

    #[test]
    fn test_secret_serde() {
        let secret = Secret::new("my-password".to_string());
        let json = serde_json::to_string(&secret).unwrap();
        assert_eq!(json, "\"[REDACTED]\"");
        assert!(!json.contains("password"));
    }
}
