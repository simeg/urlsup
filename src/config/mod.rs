//! Configuration management
//!
//! This module handles loading and managing configuration from
//! TOML files and CLI arguments.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::core::constants::{output_formats, timeouts};
use crate::core::error::Result;

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

    /// URLs to allowlist  
    pub allowlist: Option<Vec<String>>,

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

    /// Use HEAD requests instead of GET for faster validation (some servers may not support)
    pub use_head_requests: Option<bool>,

    /// Failure threshold percentage - fail only if more than X% of URLs are broken (0-100)
    pub failure_threshold: Option<f64>,

    /// Show memory usage and performance optimization suggestions
    pub show_performance: Option<bool>,

    /// Generate HTML dashboard report
    pub html_dashboard_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            timeout: Some(timeouts::DEFAULT_TIMEOUT_SECONDS),
            threads: None, // Will default to CPU core count
            allow_timeout: Some(false),
            file_types: None,
            exclude_patterns: None,
            allowlist: None,
            allowed_status_codes: None,
            user_agent: None,
            retry_attempts: Some(0),
            retry_delay: Some(timeouts::DEFAULT_RETRY_DELAY_MS),
            skip_ssl_verification: Some(false),
            proxy: None,
            rate_limit_delay: Some(timeouts::DEFAULT_RATE_LIMIT_MS),
            output_format: Some(output_formats::DEFAULT.to_string()),
            verbose: Some(false),
            use_head_requests: Some(false), // Default to GET for compatibility
            failure_threshold: None,        // No threshold by default - fail on any broken URL
            show_performance: Some(false),  // Disabled by default
            html_dashboard_path: None,      // No dashboard by default
        }
    }
}

impl Config {
    /// Load configuration from file, falling back to defaults
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            crate::core::error::UrlsUpError::Config(format!(
                "Could not read config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        let config: Config = toml::from_str(&content).map_err(|e| {
            crate::core::error::UrlsUpError::Config(format!(
                "Invalid TOML in config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Validate the loaded configuration
        config.validate()?;
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
        // Core options
        if let Some(timeout) = cli_config.timeout {
            self.timeout = Some(timeout);
        }

        // Filtering & inclusion
        if let Some(ref file_types) = cli_config.file_types {
            self.file_types = Some(file_types.clone());
        }
        if let Some(ref allowlist) = cli_config.allowlist {
            self.allowlist = Some(allowlist.clone());
        }
        if let Some(ref allowed_status_codes) = cli_config.allowed_status_codes {
            self.allowed_status_codes = Some(allowed_status_codes.clone());
        }
        if let Some(ref exclude_patterns) = cli_config.exclude_patterns {
            self.exclude_patterns = Some(exclude_patterns.clone());
        }

        // Performance & behavior
        if let Some(threads) = cli_config.threads {
            self.threads = Some(threads);
        }
        if let Some(retry_attempts) = cli_config.retry_attempts {
            self.retry_attempts = Some(retry_attempts);
        }
        if let Some(retry_delay) = cli_config.retry_delay {
            self.retry_delay = Some(retry_delay);
        }
        if let Some(rate_limit_delay) = cli_config.rate_limit_delay {
            self.rate_limit_delay = Some(rate_limit_delay);
        }
        if cli_config.allow_timeout {
            self.allow_timeout = Some(true);
        }
        if let Some(threshold) = cli_config.failure_threshold {
            self.failure_threshold = Some(threshold);
        }

        // Output & format
        if cli_config.verbose {
            self.verbose = Some(true);
        }
        if let Some(ref output_format) = cli_config.output_format {
            self.output_format = Some(output_format.clone());
        }

        // Network & security
        if let Some(ref user_agent) = cli_config.user_agent {
            self.user_agent = Some(user_agent.clone());
        }
        if let Some(ref proxy) = cli_config.proxy {
            self.proxy = Some(proxy.clone());
        }
        if cli_config.skip_ssl_verification {
            self.skip_ssl_verification = Some(true);
        }

        // Performance Analysis
        if cli_config.show_performance {
            self.show_performance = Some(true);
        }
        if let Some(ref dashboard_path) = cli_config.html_dashboard_path {
            self.html_dashboard_path = Some(dashboard_path.clone());
        }
    }

    /// Compile exclude patterns into regex objects
    pub fn compile_exclude_patterns(&self) -> Result<Vec<Regex>> {
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
        Duration::from_secs(self.timeout.unwrap_or(timeouts::DEFAULT_TIMEOUT_SECONDS))
    }

    /// Get retry delay as Duration
    pub fn retry_delay_duration(&self) -> Duration {
        Duration::from_millis(self.retry_delay.unwrap_or(1000))
    }

    /// Get rate limit delay as Duration
    pub fn rate_limit_delay_duration(&self) -> Duration {
        Duration::from_millis(self.rate_limit_delay.unwrap_or(0))
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate timeout
        if let Some(timeout) = self.timeout {
            if timeout == 0 {
                return Err(crate::core::error::UrlsUpError::Config(
                    "Timeout cannot be 0. Expected a positive integer representing seconds."
                        .to_string(),
                ));
            }
            if timeout > 86400 {
                return Err(crate::core::error::UrlsUpError::Config(format!(
                    "Timeout of {timeout} seconds is extremely large (>24 hours). Consider using a smaller value."
                )));
            }
        }

        // Validate concurrency/threads
        if let Some(threads) = self.threads {
            if threads == 0 {
                return Err(crate::core::error::UrlsUpError::Config(
                    "Thread count cannot be 0. Expected a positive integer.".to_string(),
                ));
            }
            if threads > 1000 {
                return Err(crate::core::error::UrlsUpError::Config(format!(
                    "Thread count of {threads} is extremely high and may cause system instability. Consider using a smaller value."
                )));
            }
        }

        // Validate retry attempts
        if let Some(retry) = self.retry_attempts
            && retry > 20
        {
            return Err(crate::core::error::UrlsUpError::Config(format!(
                "Retry attempts of {retry} is very high and may cause long delays. Consider using a smaller value."
            )));
        }

        // Validate status codes
        if let Some(ref codes) = self.allowed_status_codes {
            for &code in codes {
                if !(100..=599).contains(&code) {
                    return Err(crate::core::error::UrlsUpError::Config(format!(
                        "Status code {code} is not a valid HTTP status code. Expected a number between 100-599."
                    )));
                }
            }
        }

        // Validate output format
        if let Some(ref format) = self.output_format {
            match format.as_str() {
                f if output_formats::ALL.contains(&f) => {}
                _ => {
                    return Err(crate::core::error::UrlsUpError::Config(format!(
                        "Invalid output format '{format}'. Expected one of: {}.",
                        output_formats::ALL.join(", ")
                    )));
                }
            }
        }

        // Validate failure threshold
        if let Some(threshold) = self.failure_threshold {
            const EPSILON: f64 = 1e-10;
            if !(-EPSILON..=100.0 + EPSILON).contains(&threshold) {
                return Err(crate::core::error::UrlsUpError::Config(format!(
                    "Failure threshold {threshold}% is invalid. Expected a value between 0-100."
                )));
            }
        }

        // Validate exclude patterns by trying to compile them
        self.compile_exclude_patterns()?;

        Ok(())
    }
}

/// Configuration options that can come from CLI
#[derive(Debug, Default)]
pub struct CliConfig {
    // Core options
    pub timeout: Option<u64>,

    // Filtering & inclusion
    pub file_types: Option<Vec<String>>,        // --include
    pub allowlist: Option<Vec<String>>,         // --allowlist
    pub allowed_status_codes: Option<Vec<u16>>, // --allow-status
    pub exclude_patterns: Option<Vec<String>>,  // --exclude-pattern

    // Performance & behavior
    pub threads: Option<usize>,         // --concurrency (was threads)
    pub retry_attempts: Option<u8>,     // --retry
    pub retry_delay: Option<u64>,       // --retry-delay
    pub rate_limit_delay: Option<u64>,  // --rate-limit
    pub allow_timeout: bool,            // --allow-timeout
    pub failure_threshold: Option<f64>, // --failure-threshold

    // Output & format
    pub quiet: bool,                   // --quiet
    pub verbose: bool,                 // --verbose
    pub output_format: Option<String>, // --format
    pub no_progress: bool,             // --no-progress

    // Network & security
    pub user_agent: Option<String>,  // --user-agent
    pub proxy: Option<String>,       // --proxy
    pub skip_ssl_verification: bool, // --insecure

    // Configuration
    pub config_file: Option<String>, // --config
    pub no_config: bool,             // --no-config

    // Performance Analysis
    pub show_performance: bool,              // --show-performance
    pub html_dashboard_path: Option<String>, // --html-dashboard
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::constants::output_formats;
    use std::io::Write;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.timeout, Some(timeouts::DEFAULT_TIMEOUT_SECONDS));
        assert_eq!(config.allow_timeout, Some(false));
        assert_eq!(config.retry_attempts, Some(0));
        assert_eq!(
            config.output_format,
            Some(output_formats::DEFAULT.to_string())
        );
    }

    #[test]
    fn test_config_load_from_file() -> Result<()> {
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
    fn test_compile_exclude_patterns() -> Result<()> {
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

    #[test]
    fn test_compile_exclude_patterns_empty() -> Result<()> {
        let config = Config {
            exclude_patterns: None,
            ..Default::default()
        };

        let patterns = config.compile_exclude_patterns()?;
        assert_eq!(patterns.len(), 0);

        Ok(())
    }

    #[test]
    fn test_compile_exclude_patterns_invalid_regex() {
        let config = Config {
            exclude_patterns: Some(vec![r"[invalid regex".to_string()]),
            ..Default::default()
        };

        assert!(config.compile_exclude_patterns().is_err());
    }

    #[test]
    fn test_file_types_as_set() {
        let config = Config {
            file_types: Some(vec![
                "md".to_string(),
                "txt".to_string(),
                "html".to_string(),
            ]),
            ..Default::default()
        };

        let set = config.file_types_as_set().unwrap();
        assert_eq!(set.len(), 3);
        assert!(set.contains("md"));
        assert!(set.contains("txt"));
        assert!(set.contains("html"));
        assert!(!set.contains("py"));
    }

    #[test]
    fn test_file_types_as_set_none() {
        let config = Config {
            file_types: None,
            ..Default::default()
        };

        assert!(config.file_types_as_set().is_none());
    }

    #[test]
    fn test_timeout_duration() {
        let config = Config {
            timeout: Some(45),
            ..Default::default()
        };

        assert_eq!(config.timeout_duration(), Duration::from_secs(45));

        let default_config = Config {
            timeout: None,
            ..Default::default()
        };

        assert_eq!(
            default_config.timeout_duration(),
            Duration::from_secs(timeouts::DEFAULT_TIMEOUT_SECONDS)
        );
    }

    #[test]
    fn test_retry_delay_duration() {
        let config = Config {
            retry_delay: Some(2500),
            ..Default::default()
        };

        assert_eq!(config.retry_delay_duration(), Duration::from_millis(2500));

        let default_config = Config {
            retry_delay: None,
            ..Default::default()
        };

        assert_eq!(
            default_config.retry_delay_duration(),
            Duration::from_millis(1000)
        );
    }

    #[test]
    fn test_rate_limit_delay_duration() {
        let config = Config {
            rate_limit_delay: Some(500),
            ..Default::default()
        };

        assert_eq!(
            config.rate_limit_delay_duration(),
            Duration::from_millis(500)
        );

        let default_config = Config {
            rate_limit_delay: None,
            ..Default::default()
        };

        assert_eq!(
            default_config.rate_limit_delay_duration(),
            Duration::from_millis(0)
        );
    }

    #[test]
    fn test_config_load_from_standard_locations() {
        // This test ensures that the function doesn't panic even if no config file exists
        let config = Config::load_from_standard_locations();
        // Should fall back to defaults
        assert_eq!(config.timeout, Some(timeouts::DEFAULT_TIMEOUT_SECONDS));
        assert_eq!(config.allow_timeout, Some(false));
    }

    #[test]
    fn test_config_merge_with_cli_all_fields() {
        let mut config = Config::default();
        let cli_config = CliConfig {
            timeout: Some(60),
            file_types: Some(vec!["md".to_string(), "html".to_string()]),
            allowlist: Some(vec!["example.com".to_string()]),
            allowed_status_codes: Some(vec![404, 429]),
            exclude_patterns: Some(vec![r".*\.local$".to_string()]),
            threads: Some(8),
            retry_attempts: Some(3),
            retry_delay: Some(2000),
            rate_limit_delay: Some(100),
            allow_timeout: true,
            quiet: true,
            verbose: true,
            output_format: Some(output_formats::JSON.to_string()),
            no_progress: true,
            user_agent: Some("test-agent".to_string()),
            proxy: Some("http://proxy.test:8080".to_string()),
            skip_ssl_verification: true,
            config_file: Some("/path/to/config".to_string()),
            no_config: true,
            failure_threshold: Some(15.0),
            show_performance: false,
            html_dashboard_path: None,
        };

        config.merge_with_cli(&cli_config);

        assert_eq!(config.timeout, Some(60));
        assert_eq!(
            config.file_types,
            Some(vec!["md".to_string(), "html".to_string()])
        );
        assert_eq!(config.allowlist, Some(vec!["example.com".to_string()]));
        assert_eq!(config.allowed_status_codes, Some(vec![404, 429]));
        assert_eq!(
            config.exclude_patterns,
            Some(vec![r".*\.local$".to_string()])
        );
        assert_eq!(config.threads, Some(8));
        assert_eq!(config.retry_attempts, Some(3));
        assert_eq!(config.retry_delay, Some(2000));
        assert_eq!(config.rate_limit_delay, Some(100));
        assert_eq!(config.allow_timeout, Some(true));
        assert_eq!(config.verbose, Some(true));
        assert_eq!(config.output_format, Some(output_formats::JSON.to_string()));
        assert_eq!(config.user_agent, Some("test-agent".to_string()));
        assert_eq!(config.proxy, Some("http://proxy.test:8080".to_string()));
        assert_eq!(config.skip_ssl_verification, Some(true));
        assert_eq!(config.failure_threshold, Some(15.0));
    }

    #[test]
    fn test_config_load_from_file_invalid_toml() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"invalid toml content [").unwrap();

        let result = Config::load_from_file(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_config_load_from_file_nonexistent() {
        let result = Config::load_from_file("/path/that/does/not/exist.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_config_default() {
        let cli_config = CliConfig::default();
        assert_eq!(cli_config.timeout, None);
        assert_eq!(cli_config.file_types, None);
        assert_eq!(cli_config.allowlist, None);
        assert_eq!(cli_config.allowed_status_codes, None);
        assert_eq!(cli_config.exclude_patterns, None);
        assert_eq!(cli_config.threads, None);
        assert_eq!(cli_config.retry_attempts, None);
        assert_eq!(cli_config.retry_delay, None);
        assert_eq!(cli_config.rate_limit_delay, None);
        assert!(!cli_config.allow_timeout);
        assert!(!cli_config.quiet);
        assert!(!cli_config.verbose);
        assert_eq!(cli_config.output_format, None);
        assert!(!cli_config.no_progress);
        assert_eq!(cli_config.user_agent, None);
        assert_eq!(cli_config.proxy, None);
        assert!(!cli_config.skip_ssl_verification);
        assert_eq!(cli_config.config_file, None);
        assert!(!cli_config.no_config);
    }

    #[test]
    fn test_config_empty_compile_exclude_patterns() -> Result<()> {
        let config = Config {
            exclude_patterns: Some(vec![]),
            ..Default::default()
        };

        let patterns = config.compile_exclude_patterns()?;
        assert_eq!(patterns.len(), 0);

        Ok(())
    }

    #[test]
    fn test_config_validation_invalid_timeout() {
        let config = Config {
            timeout: Some(0),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = Config {
            timeout: Some(100000), // Too large
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_threads() {
        let config = Config {
            threads: Some(0),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = Config {
            threads: Some(2000), // Too many
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_retry_attempts() {
        let config = Config {
            retry_attempts: Some(50), // Too many
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_status_codes() {
        let config = Config {
            allowed_status_codes: Some(vec![50, 700]), // Invalid range
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_invalid_output_format() {
        let config = Config {
            output_format: Some("invalid".to_string()),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_valid_config() -> Result<()> {
        let config = Config {
            timeout: Some(timeouts::DEFAULT_TIMEOUT_SECONDS),
            threads: Some(4),
            retry_attempts: Some(3),
            allowed_status_codes: Some(vec![200, 404, 429]),
            output_format: Some(output_formats::JSON.to_string()),
            ..Default::default()
        };
        config.validate()?;
        Ok(())
    }

    #[test]
    fn test_config_validation_edge_case_values() -> Result<()> {
        let config = Config {
            timeout: Some(1),                           // Minimum valid
            threads: Some(1),                           // Minimum valid
            retry_attempts: Some(20),                   // Maximum valid
            allowed_status_codes: Some(vec![100, 599]), // Edge cases
            output_format: Some(output_formats::MINIMAL.to_string()),
            ..Default::default()
        };
        config.validate()?;
        Ok(())
    }

    #[test]
    fn test_config_load_from_file_with_validation() -> Result<()> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(b"timeout = 0")?; // Invalid config

        let result = Config::load_from_file(file.path());
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_config_merge_overwrites_correctly() {
        let mut config = Config {
            timeout: Some(10),
            verbose: Some(false),
            ..Default::default()
        };

        let cli_config = CliConfig {
            timeout: Some(timeouts::DEFAULT_TIMEOUT_SECONDS),
            verbose: true,
            ..Default::default()
        };

        config.merge_with_cli(&cli_config);

        assert_eq!(config.timeout, Some(timeouts::DEFAULT_TIMEOUT_SECONDS)); // Overwritten
        assert_eq!(config.verbose, Some(true)); // Overwritten
    }

    #[test]
    fn test_config_merge_preserves_unset_values() {
        let mut config = Config {
            timeout: Some(10),
            threads: Some(4),
            ..Default::default()
        };

        let cli_config = CliConfig {
            timeout: Some(timeouts::DEFAULT_TIMEOUT_SECONDS),
            // threads not set in CLI
            ..Default::default()
        };

        config.merge_with_cli(&cli_config);

        assert_eq!(config.timeout, Some(timeouts::DEFAULT_TIMEOUT_SECONDS)); // Overwritten
        assert_eq!(config.threads, Some(4)); // Preserved
    }
}
