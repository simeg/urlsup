use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Timeout in seconds for HTTP requests
    pub timeout: Option<u64>,

    /// Number of concurrent threads for validation
    pub threads: Option<usize>,

    /// Allow URLs that timeout
    pub allow_timeout: Option<bool>,

    /// File extensions to process
    pub file_types: Option<Vec<String>>,

    /// URL patterns to exclude (regex)
    pub exclude_patterns: Option<Vec<String>>,

    /// URLs to whitelist
    pub whitelist: Option<Vec<String>>,

    /// HTTP status codes to allow
    pub allowed_status_codes: Option<Vec<u16>>,

    /// Custom User-Agent header
    pub user_agent: Option<String>,

    /// Retry attempts for failed requests
    pub retry_attempts: Option<u8>,

    /// Delay between retries in milliseconds
    pub retry_delay: Option<u64>,

    /// Skip SSL certificate verification
    pub skip_ssl_verification: Option<bool>,

    /// HTTP/HTTPS proxy URL
    pub proxy: Option<String>,

    /// Rate limiting: delay between requests in milliseconds
    pub rate_limit_delay: Option<u64>,

    /// Output format (text, json)
    pub output_format: Option<String>,

    /// Enable verbose logging
    pub verbose: Option<bool>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Some(30),
            threads: None, // Will default to CPU core count
            allow_timeout: Some(false),
            file_types: None,
            exclude_patterns: None,
            whitelist: None,
            allowed_status_codes: None,
            user_agent: None,
            retry_attempts: Some(0),
            retry_delay: Some(1000),
            skip_ssl_verification: Some(false),
            proxy: None,
            rate_limit_delay: Some(0),
            output_format: Some("text".to_string()),
            verbose: Some(false),
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Try to find and load a config file in standard locations
    pub fn load_from_standard_locations() -> Self {
        // Check for .urlsup.toml in current directory
        if let Ok(config) = Self::load_from_file(".urlsup.toml") {
            return config;
        }

        // Check for .urlsup.toml in parent directories (up to 3 levels)
        for i in 1..=3 {
            let path = format!("{}.urlsup.toml", "../".repeat(i));
            if let Ok(config) = Self::load_from_file(&path) {
                return config;
            }
        }

        // Fall back to defaults
        Self::default()
    }

    /// Merge this config with CLI arguments (CLI takes precedence)
    pub fn merge_with_cli(&mut self, cli_config: &CliConfig) {
        if let Some(timeout) = cli_config.timeout {
            self.timeout = Some(timeout);
        }
        if let Some(threads) = cli_config.threads {
            self.threads = Some(threads);
        }
        if cli_config.allow_timeout {
            self.allow_timeout = Some(true);
        }
        if let Some(ref file_types) = cli_config.file_types {
            self.file_types = Some(file_types.clone());
        }
        if let Some(ref whitelist) = cli_config.whitelist {
            self.whitelist = Some(whitelist.clone());
        }
        if let Some(ref allowed_status_codes) = cli_config.allowed_status_codes {
            self.allowed_status_codes = Some(allowed_status_codes.clone());
        }
        if let Some(ref user_agent) = cli_config.user_agent {
            self.user_agent = Some(user_agent.clone());
        }
        if cli_config.skip_ssl_verification {
            self.skip_ssl_verification = Some(true);
        }
        if let Some(ref proxy) = cli_config.proxy {
            self.proxy = Some(proxy.clone());
        }
        if let Some(ref output_format) = cli_config.output_format {
            self.output_format = Some(output_format.clone());
        }
        if cli_config.verbose {
            self.verbose = Some(true);
        }
    }

    /// Compile exclude patterns into regex objects
    pub fn compile_exclude_patterns(&self) -> Result<Vec<Regex>, Box<dyn std::error::Error>> {
        let mut compiled = Vec::new();
        if let Some(ref patterns) = self.exclude_patterns {
            for pattern in patterns {
                compiled.push(Regex::new(pattern)?);
            }
        }
        Ok(compiled)
    }

    /// Convert file_types to HashSet for compatibility
    pub fn file_types_as_set(&self) -> Option<HashSet<String>> {
        self.file_types
            .as_ref()
            .map(|types| types.iter().cloned().collect())
    }

    /// Get timeout as Duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout.unwrap_or(30))
    }

    /// Get retry delay as Duration
    pub fn retry_delay_duration(&self) -> Duration {
        Duration::from_millis(self.retry_delay.unwrap_or(1000))
    }

    /// Get rate limit delay as Duration
    pub fn rate_limit_delay_duration(&self) -> Duration {
        Duration::from_millis(self.rate_limit_delay.unwrap_or(0))
    }
}

/// Configuration options that can come from CLI
#[derive(Debug, Default)]
pub struct CliConfig {
    pub timeout: Option<u64>,
    pub threads: Option<usize>,
    pub allow_timeout: bool,
    pub file_types: Option<Vec<String>>,
    pub whitelist: Option<Vec<String>>,
    pub allowed_status_codes: Option<Vec<u16>>,
    pub user_agent: Option<String>,
    pub skip_ssl_verification: bool,
    pub proxy: Option<String>,
    pub output_format: Option<String>,
    pub verbose: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.timeout, Some(30));
        assert_eq!(config.allow_timeout, Some(false));
        assert_eq!(config.retry_attempts, Some(0));
        assert_eq!(config.output_format, Some("text".to_string()));
    }

    #[test]
    fn test_config_load_from_file() -> Result<(), Box<dyn std::error::Error>> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(b"timeout = 60\nallow_timeout = true\nuser_agent = \"test-agent\"")?;

        let config = Config::load_from_file(file.path())?;
        assert_eq!(config.timeout, Some(60));
        assert_eq!(config.allow_timeout, Some(true));
        assert_eq!(config.user_agent, Some("test-agent".to_string()));

        Ok(())
    }

    #[test]
    fn test_config_merge_with_cli() {
        let mut config = Config::default();
        let cli_config = CliConfig {
            timeout: Some(45),
            allow_timeout: true,
            verbose: true,
            ..Default::default()
        };

        config.merge_with_cli(&cli_config);

        assert_eq!(config.timeout, Some(45));
        assert_eq!(config.allow_timeout, Some(true));
        assert_eq!(config.verbose, Some(true));
    }

    #[test]
    fn test_compile_exclude_patterns() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config {
            exclude_patterns: Some(vec![
                r"^https://example\.com/.*".to_string(),
                r".*\.local$".to_string(),
            ]),
            ..Default::default()
        };

        let patterns = config.compile_exclude_patterns()?;
        assert_eq!(patterns.len(), 2);

        assert!(patterns[0].is_match("https://example.com/test"));
        assert!(!patterns[0].is_match("https://other.com/test"));

        assert!(patterns[1].is_match("http://test.local"));
        assert!(!patterns[1].is_match("http://test.com"));

        Ok(())
    }
}
