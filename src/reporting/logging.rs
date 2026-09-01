use crate::config::Config;
use crate::core::constants::timeouts;
use log::{debug, error, info, warn};
use std::path::Path;

/// Pick the log level for a verbose/quiet combination.
///
/// Extracted so the precedence is testable; `init_logger` itself installs a
/// process-global logger and returns nothing, so it can only be smoke-tested.
/// Note `quiet` wins over `verbose`.
pub fn log_level_for(verbose: bool, quiet: bool) -> log::LevelFilter {
    if quiet {
        log::LevelFilter::Off
    } else if verbose {
        log::LevelFilter::Debug
    } else {
        // Structured logs are opt-in via --verbose.
        log::LevelFilter::Off
    }
}

/// Initialize the logger with appropriate level based on verbosity
pub fn init_logger(verbose: bool, quiet: bool) {
    let level = log_level_for(verbose, quiet);

    // Use try_init() to avoid panicking if logger is already initialized (e.g., in tests)
    let _ = env_logger::Builder::from_default_env()
        .filter_level(level)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .try_init();

    debug!("Logger initialized with level: {level:?}");
}

/// Log configuration information
pub fn log_config_info(config: &Config, actual_threads: usize) {
    let timeout = config.timeout.unwrap_or(timeouts::DEFAULT_TIMEOUT_SECONDS);
    let allow_timeout = config.allow_timeout.unwrap_or(false);
    let retry_attempts = config.retry_attempts.unwrap_or(0);
    let retry_delay = config
        .retry_delay
        .unwrap_or(timeouts::DEFAULT_RETRY_DELAY_MS);
    let rate_limit_delay = config.rate_limit_delay.unwrap_or(0);
    let use_head_requests = config.use_head_requests.unwrap_or(false);
    let skip_ssl_verification = config.skip_ssl_verification.unwrap_or(false);

    info!(
        "Configuration: threads={actual_threads}, timeout={timeout}s, allow_timeout={allow_timeout}"
    );
    info!("Retry: attempts={retry_attempts}, delay={retry_delay}ms");
    info!("Rate limiting: delay={rate_limit_delay}ms");
    info!("HTTP: head_requests={use_head_requests}, skip_ssl={skip_ssl_verification}");
}

/// Log file processing information
pub fn log_file_info<P: AsRef<Path>>(file_count: usize, files: &[P]) {
    info!("Processing {file_count} file(s)");
    for (i, file) in files.iter().enumerate() {
        debug!("  {}. {}", i + 1, file.as_ref().display());
    }
}

/// Log URL discovery information
pub fn log_url_discovery(unique_urls: usize, total_found: usize) {
    info!("Found {unique_urls} unique URLs (from {total_found} total)");
}

/// Log validation progress
pub fn log_validation_start(url_count: usize) {
    info!("Starting validation of {url_count} URLs");
}

/// Log validation completion
/// Number of URLs that validated cleanly.
///
/// `issues` should never exceed `url_count`, but saturate rather than
/// underflow: with overflow-checks on that panicked, and before that it
/// silently reported 18446744073709551606 valid URLs.
pub fn valid_url_count(url_count: usize, issues: usize) -> usize {
    url_count.saturating_sub(issues)
}

pub fn log_validation_complete(url_count: usize, issues: usize, duration_ms: u128) {
    let valid = valid_url_count(url_count, issues);
    if issues == 0 {
        info!(
            "✅ Validation complete: {}/{} URLs valid ({}ms)",
            valid, url_count, duration_ms
        );
    } else {
        warn!(
            "❌ Validation complete: {}/{} URLs valid, {} issues found ({}ms)",
            valid, url_count, issues, duration_ms
        );
    }
}

/// Log individual URL validation results for debugging
pub fn log_url_result(url: &str, status: Option<u16>, description: Option<&str>) {
    match (status, description) {
        (Some(status), None) => debug!("✓ {url} -> {status}"),
        (Some(status), Some(desc)) => debug!("✗ {url} -> {status} ({desc})"),
        (None, Some(desc)) => debug!("✗ {url} -> {desc}"),
        (None, None) => debug!("? {url} -> unknown"),
    }
}

/// Log error information
pub fn log_error(message: &str, source: Option<&dyn std::error::Error>) {
    match source {
        Some(err) => error!("{message}: {err}"),
        None => error!("{message}"),
    }
}

/// Log warning information
pub fn log_warning(message: &str) {
    warn!("{message}");
}

#[cfg(test)]
mod tests {
    //! Most functions here wrap `log::` macros and return `()`, so there is
    //! nothing to assert without installing a log-capture harness. Those tests
    //! are deliberately smoke tests -- they prove the call path does not panic
    //! on the inputs given. Logic that *is* assertable (log level selection)
    //! is tested directly via `log_level_for`.

    use super::*;
    use std::io;

    #[test]
    fn test_log_level_for_precedence() {
        use log::LevelFilter;

        // quiet wins over verbose
        assert_eq!(log_level_for(true, true), LevelFilter::Off);
        assert_eq!(log_level_for(false, true), LevelFilter::Off);
        // verbose alone enables debug logging
        assert_eq!(log_level_for(true, false), LevelFilter::Debug);
        // neither: structured logs are opt-in
        assert_eq!(log_level_for(false, false), LevelFilter::Off);
    }

    #[test]
    fn test_init_logger_is_idempotent() {
        // `init_logger` installs a process-global logger via `try_init()` and
        // returns nothing, so the only assertable property is that repeated
        // calls do not panic. The level-selection logic it delegates to is
        // covered by `test_log_level_for_precedence` above.
        init_logger(true, false);
        init_logger(false, true);
        init_logger(false, false);
        init_logger(true, true);
    }

    #[test]
    fn test_log_config_info_all_params() {
        // Test with various parameter combinations
        let config1 = Config {
            timeout: Some(5),
            allow_timeout: Some(false),
            retry_attempts: Some(3),
            retry_delay: Some(1000),
            rate_limit_delay: Some(100),
            use_head_requests: Some(false),
            skip_ssl_verification: Some(false),
            ..Default::default()
        };
        log_config_info(&config1, 4);

        let config2 = Config {
            timeout: Some(60),
            allow_timeout: Some(true),
            retry_attempts: Some(5),
            retry_delay: Some(2000),
            rate_limit_delay: Some(200),
            use_head_requests: Some(true),
            skip_ssl_verification: Some(true),
            ..Default::default()
        };
        log_config_info(&config2, 8);

        let config3 = Config {
            timeout: Some(10),
            allow_timeout: Some(false),
            retry_attempts: Some(0),
            retry_delay: Some(0),
            rate_limit_delay: Some(0),
            use_head_requests: Some(false),
            skip_ssl_verification: Some(false),
            ..Default::default()
        };
        log_config_info(&config3, 1);
    }

    #[test]
    fn test_log_config_info_edge_cases() {
        // Test with edge case values
        let config1 = Config {
            timeout: Some(1),
            allow_timeout: Some(true),
            retry_attempts: Some(0),
            retry_delay: Some(0),
            rate_limit_delay: Some(0),
            use_head_requests: Some(false),
            skip_ssl_verification: Some(false),
            ..Default::default()
        };
        log_config_info(&config1, 1);

        let config2 = Config {
            timeout: Some(86400),
            allow_timeout: Some(false),
            retry_attempts: Some(255),
            retry_delay: Some(u64::MAX),
            rate_limit_delay: Some(u64::MAX),
            use_head_requests: Some(true),
            skip_ssl_verification: Some(true),
            ..Default::default()
        };
        log_config_info(&config2, 1000);
    }

    #[test]
    fn test_log_config_info_with_defaults() {
        // Test with default config (all None values)
        let config = Config::default();
        log_config_info(&config, 4);

        // Test with partially filled config
        let config_partial = Config {
            timeout: Some(45),
            use_head_requests: Some(true),
            ..Default::default()
        };
        log_config_info(&config_partial, 8);
    }

    #[test]
    fn test_log_file_info_empty() {
        let empty_files: Vec<String> = vec![];
        log_file_info(0, &empty_files);
    }

    #[test]
    fn test_log_file_info_single_file() {
        log_file_info(1, &["single.md".to_string()]);
    }

    #[test]
    fn test_log_file_info_multiple_files() {
        let files = vec![
            "file1.md".to_string(),
            "file2.txt".to_string(),
            "file3.html".to_string(),
        ];
        log_file_info(3, &files);
    }

    #[test]
    fn test_log_file_info_path_buf() {
        use std::path::PathBuf;
        let paths = vec![
            PathBuf::from("path/to/file1.md"),
            PathBuf::from("another/file2.txt"),
        ];
        log_file_info(2, &paths);
    }

    #[test]
    fn test_log_url_discovery_zero() {
        log_url_discovery(0, 0);
    }

    #[test]
    fn test_log_url_discovery_deduplication() {
        log_url_discovery(5, 10); // 5 unique from 10 total
        log_url_discovery(10, 10); // No duplicates
        log_url_discovery(1, 100); // Heavy deduplication
    }

    #[test]
    fn test_log_validation_start_zero() {
        log_validation_start(0);
    }

    #[test]
    fn test_log_validation_start_large() {
        log_validation_start(1000);
    }

    #[test]
    fn test_log_validation_complete_all_success() {
        log_validation_complete(10, 0, 1000); // All successful
    }

    #[test]
    fn test_log_validation_complete_all_failed() {
        // Regression: `issues > url_count` underflowed `url_count - issues`.
        // With overflow-checks on this panicked; before that it printed
        // 18446744073709551606 valid URLs.
        log_validation_complete(0, 10, 2000);

        // More issues than URLs saturates at zero instead of underflowing.
        assert_eq!(valid_url_count(0, 10), 0);
        assert_eq!(valid_url_count(3, 10), 0);
        // Normal cases are unaffected.
        assert_eq!(valid_url_count(10, 3), 7);
        assert_eq!(valid_url_count(10, 0), 10);
        assert_eq!(valid_url_count(0, 0), 0);
    }

    #[test]
    fn test_log_validation_complete_mixed() {
        log_validation_complete(7, 3, 1500); // Mixed results
    }

    #[test]
    fn test_log_validation_complete_zero_time() {
        log_validation_complete(5, 2, 0); // Zero duration
    }

    #[test]
    fn test_log_validation_complete_long_time() {
        log_validation_complete(100, 5, 30000); // Long duration
    }

    #[test]
    fn test_log_url_result_success() {
        log_url_result("https://example.com", Some(200), None);
        log_url_result("https://test.org", Some(201), None);
        log_url_result("https://api.service.com", Some(204), None);
    }

    #[test]
    fn test_log_url_result_client_errors() {
        log_url_result("https://example.com/404", Some(404), Some("Not Found"));
        log_url_result("https://example.com/403", Some(403), Some("Forbidden"));
        log_url_result(
            "https://example.com/429",
            Some(429),
            Some("Too Many Requests"),
        );
    }

    #[test]
    fn test_log_url_result_server_errors() {
        log_url_result(
            "https://example.com/500",
            Some(500),
            Some("Internal Server Error"),
        );
        log_url_result("https://example.com/502", Some(502), Some("Bad Gateway"));
    }

    #[test]
    fn test_log_url_result_network_errors() {
        log_url_result("https://unreachable.test", None, Some("Connection timeout"));
        log_url_result("https://dns.failure", None, Some("DNS resolution failed"));
        log_url_result("https://ssl.invalid", None, Some("SSL certificate error"));
    }

    #[test]
    fn test_log_url_result_unknown_state() {
        log_url_result("https://unknown.state", None, None);
    }

    #[test]
    fn test_log_error_with_source() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        log_error("Failed to read file", Some(&io_error));
    }

    #[test]
    fn test_log_error_without_source() {
        log_error("Something went wrong", None);
        log_error("Configuration error", None);
    }

    #[test]
    fn test_log_warning_various_messages() {
        log_warning("This is a warning");
        log_warning("Deprecated feature used");
        log_warning("Performance concern detected");
        log_warning("Configuration fallback used");
    }

    #[test]
    fn test_log_functions_with_special_characters() {
        // Test that logging functions handle special characters correctly
        log_url_result("https://example.com/path with spaces", Some(200), None);
        log_url_result("https://example.com/ünïcödé", Some(200), None);
        log_error("Error with special chars: äöü ñ", None);
        log_warning("Warning with emojis: ⚠️ 🔥");
    }

    #[test]
    fn test_log_functions_with_long_strings() {
        let long_url = "https://example.com/".to_string() + &"very-long-path/".repeat(100);
        log_url_result(&long_url, Some(200), None);

        let long_error = "This is a very long error message: ".to_string() + &"error ".repeat(100);
        log_error(&long_error, None);

        let long_warning = "Long warning: ".to_string() + &"warning ".repeat(50);
        log_warning(&long_warning);
    }

    #[test]
    fn test_log_functions_with_empty_strings() {
        log_url_result("", Some(200), None);
        log_url_result("https://example.com", Some(200), Some(""));
        log_error("", None);
        log_warning("");
    }

    #[test]
    fn test_logging_functions_are_callable_from_multiple_threads() {
        // The previous version of this test spawned 4 threads doing 30 log
        // calls each and asserted nothing -- it could not detect a data race,
        // and would have passed even if the functions were empty. What is
        // actually worth pinning is that these take no non-Sync state, which
        // the compiler can prove at compile time.
        fn assert_callable_concurrently<F: Fn() + Send + Sync>(_f: F) {}

        assert_callable_concurrently(|| log_warning("probe"));
        assert_callable_concurrently(|| log_error("probe", None));
        assert_callable_concurrently(|| log_url_result("https://example.com", Some(200), None));
    }
}
