// Command-line interface definitions and parsing for urlsup

use crate::config::CliConfig;
use crate::core::constants::{output_formats, timeouts};
use crate::core::error::{Result, UrlsUpError};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = None,
    help_template = "\
{before-help}{name} - CLI to validate URLs in files [version {version}]

{usage-heading} {usage}

Example:

	$ urlsup . --recursive --include md,txt

{all-args}{after-help}
"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Files or directories to check
    pub files: Vec<String>,

    // Core Options
    /// Recursively process directories. Will skip files/directories listed in .gitignore
    #[arg(short = 'r', long, help_heading = "Core Options")]
    pub recursive: bool,

    /// Connection timeout in seconds (default: 5)
    #[arg(
        short = 't',
        long,
        value_name = "SECONDS",
        help_heading = "Core Options"
    )]
    pub timeout: Option<u64>,

    /// Concurrent requests (default: CPU cores)
    #[arg(long, value_name = "COUNT", help_heading = "Core Options")]
    pub concurrency: Option<usize>,

    // Filtering & Content
    /// File extensions to process (e.g., md,html,txt)
    #[arg(long, value_name = "EXTENSIONS", help_heading = "Filtering & Content")]
    pub include: Option<String>,

    /// Hosts or URL prefixes to allow (comma-separated)
    #[arg(long, value_name = "URLS", help_heading = "Filtering & Content")]
    pub allowlist: Option<String>,

    /// Status codes to allow (comma-separated)
    #[arg(long, value_name = "CODES", help_heading = "Filtering & Content")]
    pub allow_status: Option<String>,

    /// URL patterns to exclude (regex)
    #[arg(long, value_name = "REGEX", help_heading = "Filtering & Content")]
    pub exclude_pattern: Vec<String>,

    /// Do not respect .gitignore/.ignore files when recursing
    #[arg(long, help_heading = "Filtering & Content")]
    pub no_ignore: bool,

    // Retry & Rate Limiting
    /// Retry attempts for failed requests (default: 0)
    #[arg(long, value_name = "COUNT", help_heading = "Retry & Rate Limiting")]
    pub retry: Option<u8>,

    /// Delay between retries in ms (default: 1000)
    #[arg(long, value_name = "MS", help_heading = "Retry & Rate Limiting")]
    pub retry_delay: Option<u64>,

    /// Delay between requests in ms (default: 0)
    #[arg(long, value_name = "MS", help_heading = "Retry & Rate Limiting")]
    pub rate_limit: Option<u64>,

    /// Allow URLs that timeout
    #[arg(long, help_heading = "Retry & Rate Limiting")]
    pub allow_timeout: bool,

    /// Do not allow URLs that timeout (overrides config file)
    #[arg(
        long,
        conflicts_with = "allow_timeout",
        help_heading = "Retry & Rate Limiting"
    )]
    pub no_allow_timeout: bool,

    /// Fail only if more than X% URLs are broken (0-100)
    #[arg(long, value_name = "PERCENT", help_heading = "Retry & Rate Limiting")]
    pub failure_threshold: Option<f64>,

    // Output & Verbosity
    /// Suppress progress output
    #[arg(short = 'q', long, help_heading = "Output & Verbosity")]
    pub quiet: bool,

    /// Enable verbose logging
    #[arg(short = 'v', long, help_heading = "Output & Verbosity")]
    pub verbose: bool,

    /// Output format
    #[arg(long, value_name = "FORMAT", value_parser = output_formats::ALL, default_value = output_formats::DEFAULT, help_heading = "Output & Verbosity")]
    pub format: String,

    /// Disable progress bars
    #[arg(long, help_heading = "Output & Verbosity")]
    pub no_progress: bool,

    // Network & Security
    /// Custom User-Agent header
    #[arg(long, value_name = "AGENT", help_heading = "Network & Security")]
    pub user_agent: Option<String>,

    /// HTTP/HTTPS proxy URL
    #[arg(long, value_name = "URL", help_heading = "Network & Security")]
    pub proxy: Option<String>,

    /// Skip SSL certificate verification
    #[arg(long, help_heading = "Network & Security")]
    pub insecure: bool,

    /// Enforce SSL certificate verification (overrides config file)
    #[arg(long, conflicts_with = "insecure", help_heading = "Network & Security")]
    pub no_insecure: bool,

    // Configuration
    /// Use specific config file
    #[arg(long, value_name = "FILE", help_heading = "Configuration")]
    pub config: Option<String>,

    /// Ignore config files
    #[arg(long, help_heading = "Configuration")]
    pub no_config: bool,

    // Performance Analysis
    /// Show memory usage and optimization suggestions
    #[arg(long, help_heading = "Performance Analysis")]
    pub show_performance: bool,

    /// Do not show memory usage and optimization suggestions (overrides config file)
    #[arg(
        long,
        conflicts_with = "show_performance",
        help_heading = "Performance Analysis"
    )]
    pub no_show_performance: bool,

    /// Generate HTML dashboard report
    #[arg(long, value_name = "PATH", help_heading = "Performance Analysis")]
    pub html_dashboard: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate shell completions
    #[command(name = "completion-generate", arg_required_else_help = true)]
    CompletionGenerate {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Install shell completions to standard location
    #[command(name = "completion-install", arg_required_else_help = true)]
    CompletionInstall {
        /// The shell to install completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Run interactive configuration wizard
    #[command(name = "config-wizard")]
    ConfigWizard,
}

/// Convert derive-based CLI arguments directly to CliConfig structure
pub fn cli_to_config(cli: &Cli) -> Result<CliConfig> {
    let mut cli_config = CliConfig::default();

    // Core options
    if let Some(timeout) = cli.timeout {
        if timeout == 0 {
            return Err(UrlsUpError::Config(
                "Timeout cannot be 0. Expected a positive integer representing seconds."
                    .to_string(),
            ));
        }
        if timeout > timeouts::MAX_TIMEOUT_SECONDS {
            eprintln!(
                "Warning: Timeout of {timeout} seconds is quite large. Consider using a smaller value for better user experience."
            );
        }
        cli_config.timeout = Some(timeout);
    }

    // Filtering & inclusion
    if let Some(ref include_str) = cli.include {
        cli_config.file_types = Some(
            include_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    if let Some(ref allowlist_str) = cli.allowlist {
        cli_config.allowlist = Some(
            allowlist_str
                .split(',')
                .filter_map(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.trim().to_string())
                    }
                })
                .collect(),
        );
    }

    if let Some(ref status_str) = cli.allow_status {
        let mut codes = Vec::new();
        for raw in status_str.split(',') {
            let entry = raw.trim();
            if entry.is_empty() {
                continue;
            }
            // Message wording matches the pre-refactor output; `tests/cli.rs`
            // asserts on it, and it is user-facing.
            let code: u16 = entry.parse().map_err(|_| {
                UrlsUpError::Config(format!(
                    "Status code '{entry}' is not a valid HTTP status code. Expected a number between 100-599."
                ))
            })?;
            if !(100..=599).contains(&code) {
                return Err(UrlsUpError::Config(format!(
                    "Status code '{code}' is not a valid HTTP status code. Expected a number between 100-599."
                )));
            }
            codes.push(code);
        }
        cli_config.allowed_status_codes = Some(codes);
    }

    if !cli.exclude_pattern.is_empty() {
        cli_config.exclude_patterns = Some(cli.exclude_pattern.clone());
    }

    cli_config.no_ignore = cli.no_ignore;

    // Performance & behavior
    if let Some(concurrency) = cli.concurrency {
        if concurrency == 0 {
            return Err(UrlsUpError::Config(
                "Concurrency cannot be 0. Expected a positive integer representing the number of concurrent requests."
                    .to_string(),
            ));
        }
        if concurrency > 100 {
            eprintln!(
                "Warning: Concurrency of {concurrency} is quite high and may overwhelm servers. Consider using a smaller value."
            );
        }
        cli_config.threads = Some(concurrency);
    }

    if let Some(retry) = cli.retry {
        cli_config.retry_attempts = Some(retry);
    }

    if let Some(retry_delay) = cli.retry_delay {
        cli_config.retry_delay = Some(retry_delay);
    }

    if let Some(rate_limit) = cli.rate_limit {
        cli_config.rate_limit_delay = Some(rate_limit);
    }

    cli_config.allow_timeout = cli.allow_timeout;
    cli_config.no_allow_timeout = cli.no_allow_timeout;

    // Parse failure threshold
    if let Some(threshold) = cli.failure_threshold {
        if !(0.0..=100.0).contains(&threshold) {
            return Err(UrlsUpError::Config(format!(
                "Failure threshold {threshold}% is invalid. Expected a value between 0-100."
            )));
        }
        cli_config.failure_threshold = Some(threshold);
    }

    // Output & format
    cli_config.quiet = cli.quiet;
    cli_config.verbose = cli.verbose;
    cli_config.no_progress = cli.no_progress;
    cli_config.output_format = Some(cli.format.clone());

    // Network & security
    cli_config.user_agent = cli.user_agent.clone();
    cli_config.proxy = cli.proxy.clone();
    cli_config.skip_ssl_verification = cli.insecure;
    cli_config.no_skip_ssl_verification = cli.no_insecure;

    // Configuration
    cli_config.config_file = cli.config.clone();
    cli_config.no_config = cli.no_config;

    // Performance Analysis
    cli_config.show_performance = cli.show_performance;
    cli_config.no_show_performance = cli.no_show_performance;
    cli_config.html_dashboard_path = cli.html_dashboard.clone();

    Ok(cli_config)
}

/// Validate CLI arguments using the derive-based CLI structure
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::constants::output_formats;

    fn create_default_cli() -> Cli {
        Cli {
            command: None,
            files: vec![],
            recursive: false,
            timeout: None,
            concurrency: None,
            include: None,
            allowlist: None,
            allow_status: None,
            exclude_pattern: vec![],
            no_ignore: false,
            retry: None,
            retry_delay: None,
            rate_limit: None,
            allow_timeout: false,
            no_allow_timeout: false,
            failure_threshold: None,
            quiet: false,
            verbose: false,
            format: output_formats::DEFAULT.to_string(),
            no_progress: false,
            user_agent: None,
            proxy: None,
            insecure: false,
            no_insecure: false,
            config: None,
            no_config: false,
            show_performance: false,
            no_show_performance: false,
            html_dashboard: None,
        }
    }

    #[test]
    fn test_cli_to_config_default() {
        let cli = create_default_cli();

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert_eq!(config.timeout, None);
        assert_eq!(config.threads, None);
        assert_eq!(config.file_types, None);
        assert_eq!(config.allowlist, None);
        assert_eq!(config.allowed_status_codes, None);
        assert_eq!(config.exclude_patterns, None);
        assert_eq!(config.retry_attempts, None);
        assert_eq!(config.retry_delay, None);
        assert_eq!(config.rate_limit_delay, None);
        assert!(!config.allow_timeout);
        assert_eq!(config.failure_threshold, None);
        assert!(!config.quiet);
        assert!(!config.verbose);
        assert!(!config.no_progress);
        assert_eq!(
            config.output_format,
            Some(output_formats::DEFAULT.to_string())
        );
        assert_eq!(config.user_agent, None);
        assert_eq!(config.proxy, None);
        assert!(!config.skip_ssl_verification);
        assert_eq!(config.config_file, None);
        assert!(!config.no_config);
    }

    #[test]
    fn test_cli_to_config_all_options() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.recursive = true;
        cli.timeout = Some(60);
        cli.concurrency = Some(8);
        cli.include = Some("md,txt".to_string());
        cli.allowlist = Some("example.com,google.com".to_string());
        cli.allow_status = Some("200,404".to_string());
        cli.exclude_pattern = vec![".*test.*".to_string(), ".*debug.*".to_string()];
        cli.retry = Some(3);
        cli.retry_delay = Some(2000);
        cli.rate_limit = Some(100);
        cli.allow_timeout = true;
        cli.failure_threshold = Some(10.5);
        cli.quiet = true;
        cli.verbose = true;
        cli.format = output_formats::JSON.to_string();
        cli.no_progress = true;
        cli.user_agent = Some("CustomAgent/1.0".to_string());
        cli.proxy = Some("http://proxy:8080".to_string());
        cli.insecure = true;
        cli.config = Some("config.toml".to_string());
        cli.no_config = true;

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert_eq!(config.timeout, Some(60));
        assert_eq!(config.threads, Some(8));
        assert_eq!(
            config.file_types,
            Some(vec!["md".to_string(), "txt".to_string()])
        );
        assert_eq!(
            config.allowlist,
            Some(vec!["example.com".to_string(), "google.com".to_string()])
        );
        assert_eq!(config.allowed_status_codes, Some(vec![200, 404]));
        assert_eq!(
            config.exclude_patterns,
            Some(vec![".*test.*".to_string(), ".*debug.*".to_string()])
        );
        assert_eq!(config.retry_attempts, Some(3));
        assert_eq!(config.retry_delay, Some(2000));
        assert_eq!(config.rate_limit_delay, Some(100));
        assert!(config.allow_timeout);
        assert_eq!(config.failure_threshold, Some(10.5));
        assert!(config.quiet);
        assert!(config.verbose);
        assert!(config.no_progress);
        assert_eq!(config.output_format, Some(output_formats::JSON.to_string()));
        assert_eq!(config.user_agent, Some("CustomAgent/1.0".to_string()));
        assert_eq!(config.proxy, Some("http://proxy:8080".to_string()));
        assert!(config.skip_ssl_verification);
        assert_eq!(config.config_file, Some("config.toml".to_string()));
        assert!(config.no_config);
    }

    #[test]
    fn test_cli_to_config_empty_strings() {
        let mut cli = create_default_cli();
        cli.include = Some("".to_string());
        cli.allowlist = Some("".to_string());
        cli.allow_status = Some("".to_string());
        cli.format = output_formats::MINIMAL.to_string();
        cli.user_agent = Some("".to_string());
        cli.proxy = Some("".to_string());
        cli.config = Some("".to_string());

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert_eq!(config.file_types, Some(vec!["".to_string()]));
        assert_eq!(config.allowlist, Some(vec![])); // Empty strings filtered out
        assert_eq!(config.allowed_status_codes, Some(vec![])); // Empty strings filtered out
        assert_eq!(
            config.output_format,
            Some(output_formats::MINIMAL.to_string())
        );
        assert_eq!(config.user_agent, Some("".to_string()));
        assert_eq!(config.proxy, Some("".to_string()));
        assert_eq!(config.config_file, Some("".to_string()));
    }

    #[test]
    fn test_cli_to_config_whitespace_trimming() {
        let mut cli = create_default_cli();
        cli.include = Some("  md  ,  txt  ".to_string());
        cli.allowlist = Some("  example.com  ,  google.com  ".to_string());
        cli.allow_status = Some("  200  ,  404  ".to_string());

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert_eq!(
            config.file_types,
            Some(vec!["md".to_string(), "txt".to_string()])
        );
        assert_eq!(
            config.allowlist,
            Some(vec!["example.com".to_string(), "google.com".to_string()])
        );
        assert_eq!(config.allowed_status_codes, Some(vec![200, 404]));
    }

    #[test]
    fn test_cli_to_config_mixed_empty_values() {
        let mut cli = create_default_cli();
        cli.allowlist = Some("example.com, , google.com".to_string());
        cli.allow_status = Some("200, , 404".to_string());

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert_eq!(
            config.allowlist,
            Some(vec!["example.com".to_string(), "google.com".to_string()])
        );
        assert_eq!(config.allowed_status_codes, Some(vec![200, 404]));
    }

    #[test]
    fn test_cli_to_config_boundary_values() {
        let mut cli = create_default_cli();
        cli.timeout = Some(1);
        cli.concurrency = Some(1);
        cli.allow_status = Some("100,599".to_string());
        cli.retry = Some(0);
        cli.retry_delay = Some(0);
        cli.rate_limit = Some(0);
        cli.failure_threshold = Some(0.0);

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert_eq!(config.timeout, Some(1));
        assert_eq!(config.threads, Some(1));
        assert_eq!(config.allowed_status_codes, Some(vec![100, 599]));
        assert_eq!(config.retry_attempts, Some(0));
        assert_eq!(config.retry_delay, Some(0));
        assert_eq!(config.rate_limit_delay, Some(0));
        assert_eq!(config.failure_threshold, Some(0.0));
    }

    #[test]
    fn test_cli_to_config_edge_case_failure_threshold() {
        let mut cli = create_default_cli();
        cli.failure_threshold = Some(100.0);

        let config = cli_to_config(&cli).expect("valid CLI args");
        assert_eq!(config.failure_threshold, Some(100.0));
    }

    #[test]
    fn test_cli_to_config_negating_flags() {
        let mut cli = create_default_cli();
        cli.no_allow_timeout = true;
        cli.no_insecure = true;
        cli.no_show_performance = true;

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert!(config.no_allow_timeout);
        assert!(config.no_skip_ssl_verification);
        assert!(config.no_show_performance);
        // Positive counterparts stay unset
        assert!(!config.allow_timeout);
        assert!(!config.skip_ssl_verification);
        assert!(!config.show_performance);
    }

    #[test]
    fn test_cli_to_config_negating_flags_default_false() {
        let cli = create_default_cli();

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert!(!config.no_allow_timeout);
        assert!(!config.no_skip_ssl_verification);
        assert!(!config.no_show_performance);
    }

    #[test]
    fn test_cli_parses_negating_flags() {
        let cli = Cli::try_parse_from([
            "urlsup",
            "README.md",
            "--no-allow-timeout",
            "--no-insecure",
            "--no-show-performance",
        ])
        .expect("negating flags should parse on their own");

        assert!(cli.no_allow_timeout);
        assert!(cli.no_insecure);
        assert!(cli.no_show_performance);
    }

    #[test]
    fn test_cli_rejects_conflicting_allow_timeout_flags() {
        let result = Cli::try_parse_from([
            "urlsup",
            "README.md",
            "--allow-timeout",
            "--no-allow-timeout",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn test_cli_rejects_conflicting_insecure_flags() {
        let result = Cli::try_parse_from(["urlsup", "README.md", "--insecure", "--no-insecure"]);

        assert!(result.is_err());
    }

    #[test]
    fn test_cli_rejects_conflicting_show_performance_flags() {
        let result = Cli::try_parse_from([
            "urlsup",
            "README.md",
            "--show-performance",
            "--no-show-performance",
        ]);

        assert!(result.is_err());
    }

    #[test]
    fn test_cli_to_config_no_ignore() {
        let mut cli = create_default_cli();
        cli.no_ignore = true;

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert!(config.no_ignore);
    }

    #[test]
    fn test_cli_to_config_no_ignore_default_false() {
        let cli = create_default_cli();

        let config = cli_to_config(&cli).expect("valid CLI args");

        assert!(!config.no_ignore);
    }

    #[test]
    fn test_cli_parses_no_ignore() {
        let cli = Cli::try_parse_from(["urlsup", ".", "--recursive", "--no-ignore"])
            .expect("--no-ignore should parse");

        assert!(cli.no_ignore);
    }

    #[test]
    fn test_cli_to_config_rejects_zero_timeout() {
        // Previously this called process::exit(1) from inside a library
        // function, so the failure path could not be tested at all.
        let mut cli = create_default_cli();
        cli.timeout = Some(0);

        let err = cli_to_config(&cli).expect_err("timeout 0 must be rejected");
        assert!(err.to_string().contains("Timeout cannot be 0"), "{err}");
    }

    #[test]
    fn test_cli_to_config_rejects_zero_concurrency() {
        let mut cli = create_default_cli();
        cli.concurrency = Some(0);

        let err = cli_to_config(&cli).expect_err("concurrency 0 must be rejected");
        assert!(err.to_string().contains("Concurrency cannot be 0"), "{err}");
    }

    #[test]
    fn test_cli_to_config_rejects_out_of_range_status_code() {
        for bad in ["99", "600", "1000"] {
            let mut cli = create_default_cli();
            cli.allow_status = Some(bad.to_string());

            let Err(err) = cli_to_config(&cli) else {
                panic!("status {bad} must be rejected");
            };
            assert!(
                err.to_string().contains("not a valid HTTP status code"),
                "{err}"
            );
        }
    }

    #[test]
    fn test_cli_to_config_rejects_non_numeric_status_code() {
        let mut cli = create_default_cli();
        cli.allow_status = Some("200,abc".to_string());

        let err = cli_to_config(&cli).expect_err("non-numeric status must be rejected");
        let msg = err.to_string();
        assert!(msg.contains("not a valid HTTP status code"), "{msg}");
        assert!(
            msg.contains("abc"),
            "message should name the bad entry: {msg}"
        );
    }

    #[test]
    fn test_cli_to_config_accepts_status_code_boundaries() {
        let mut cli = create_default_cli();
        cli.allow_status = Some("100,599".to_string());

        let config = cli_to_config(&cli).expect("100 and 599 are valid");
        assert_eq!(config.allowed_status_codes, Some(vec![100, 599]));
    }

    #[test]
    fn test_cli_to_config_skips_empty_status_entries() {
        let mut cli = create_default_cli();
        cli.allow_status = Some("200, , 404,".to_string());

        let config = cli_to_config(&cli).expect("empty entries are skipped");
        assert_eq!(config.allowed_status_codes, Some(vec![200, 404]));
    }

    #[test]
    fn test_cli_to_config_rejects_out_of_range_failure_threshold() {
        for bad in [-1.0, 100.1, 1000.0] {
            let mut cli = create_default_cli();
            cli.failure_threshold = Some(bad);

            let err = cli_to_config(&cli).expect_err("threshold must be 0-100");
            assert!(err.to_string().contains("Failure threshold"), "{err}");
        }
    }

    #[test]
    fn test_cli_to_config_accepts_failure_threshold_boundaries() {
        for good in [0.0, 50.0, 100.0] {
            let mut cli = create_default_cli();
            cli.failure_threshold = Some(good);

            let config = cli_to_config(&cli).expect("0-100 inclusive is valid");
            assert_eq!(config.failure_threshold, Some(good));
        }
    }

    #[test]
    fn test_cli_to_config_large_timeout_is_a_warning_not_an_error() {
        // Over MAX_TIMEOUT_SECONDS warns on stderr but must still succeed.
        let mut cli = create_default_cli();
        cli.timeout = Some(timeouts::MAX_TIMEOUT_SECONDS + 1);

        let config = cli_to_config(&cli).expect("a large timeout is allowed");
        assert_eq!(config.timeout, Some(timeouts::MAX_TIMEOUT_SECONDS + 1));
    }

    #[test]
    fn test_cli_to_config_high_concurrency_is_a_warning_not_an_error() {
        let mut cli = create_default_cli();
        cli.concurrency = Some(150);

        let config = cli_to_config(&cli).expect("high concurrency is allowed");
        assert_eq!(config.threads, Some(150));
    }
}
