// Command-line interface definitions and parsing for urlsup

use crate::config::CliConfig;
use crate::core::constants::{output_formats, timeouts};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Files or directories to check
    pub files: Vec<String>,

    // Core Options
    /// Recursively process directories
    #[arg(short = 'r', long, help_heading = "Core Options")]
    pub recursive: bool,

    /// Connection timeout in seconds (default: 30)
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

    /// URLs to allow (comma-separated)
    #[arg(long, value_name = "URLS", help_heading = "Filtering & Content")]
    pub allowlist: Option<String>,

    /// Status codes to allow (comma-separated)
    #[arg(long, value_name = "CODES", help_heading = "Filtering & Content")]
    pub allow_status: Option<String>,

    /// URL patterns to exclude (regex)
    #[arg(long, value_name = "REGEX", help_heading = "Filtering & Content")]
    pub exclude_pattern: Vec<String>,

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

    /// Failure threshold - fail only if more than X% of URLs are broken (0-100)
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

/// Parse command line arguments into CliConfig structure
pub fn parse_cli_args(matches: &clap::ArgMatches) -> CliConfig {
    let mut cli_config = CliConfig::default();

    // Core options
    if let Some(timeout_str) = matches.get_one::<String>("timeout") {
        let timeout: u64 = timeout_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Timeout '{timeout_str}' is not a valid number. Expected a positive integer representing seconds.");
            std::process::exit(1);
        });
        if timeout == 0 {
            eprintln!(
                "Error: Timeout cannot be 0. Expected a positive integer representing seconds."
            );
            std::process::exit(1);
        }
        if timeout > timeouts::MAX_TIMEOUT_SECONDS {
            eprintln!(
                "Warning: Timeout of {timeout} seconds is quite large. Consider using a smaller value for better user experience."
            );
        }
        cli_config.timeout = Some(timeout);
    }

    // Filtering & inclusion
    if let Some(include_str) = matches.get_one::<String>("include") {
        cli_config.file_types = Some(
            include_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    if let Some(allowlist_str) = matches.get_one::<String>("allowlist") {
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

    if let Some(status_str) = matches.get_one::<String>("allow-status") {
        cli_config.allowed_status_codes = Some(
            status_str
                .split(',')
                .filter_map(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        s.trim()
                            .parse::<u16>()
                            .map_err(|_| {
                                eprintln!(
                                    "Error: Status code '{s}' is not a valid HTTP status code. Expected a number between 100-599."
                                );
                                std::process::exit(1);
                            })
                            .inspect(|&code| {
                                if !(100..=599).contains(&code) {
                                    eprintln!(
                                        "Error: Status code '{code}' is not a valid HTTP status code. Expected a number between 100-599."
                                    );
                                    std::process::exit(1);
                                }
                            })
                            .ok()
                    }
                })
                .collect(),
        );
    }

    if let Some(patterns) = matches.get_many::<String>("exclude-pattern") {
        cli_config.exclude_patterns = Some(patterns.cloned().collect());
    }

    // Performance & behavior
    if let Some(concurrency_str) = matches.get_one::<String>("concurrency") {
        let concurrency: usize = concurrency_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Concurrency '{concurrency_str}' is not a valid number. Expected a positive integer representing the number of concurrent requests.");
            std::process::exit(1);
        });
        if concurrency == 0 {
            eprintln!(
                "Error: Concurrency cannot be 0. Expected a positive integer representing the number of concurrent requests."
            );
            std::process::exit(1);
        }
        if concurrency > 100 {
            eprintln!(
                "Warning: Concurrency of {concurrency} is quite high and may overwhelm servers. Consider using a smaller value."
            );
        }
        cli_config.threads = Some(concurrency);
    }

    if let Some(retry_str) = matches.get_one::<String>("retry") {
        cli_config.retry_attempts = Some(retry_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Retry count '{retry_str}' is not a valid number. Expected a non-negative integer representing the number of retry attempts.");
            std::process::exit(1);
        }));
    }

    if let Some(retry_delay_str) = matches.get_one::<String>("retry-delay") {
        cli_config.retry_delay = Some(retry_delay_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Retry delay '{retry_delay_str}' is not a valid number. Expected a non-negative integer representing milliseconds.");
            std::process::exit(1);
        }));
    }

    if let Some(rate_limit_str) = matches.get_one::<String>("rate-limit") {
        cli_config.rate_limit_delay = Some(rate_limit_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Rate limit '{rate_limit_str}' is not a valid number. Expected a non-negative integer representing milliseconds between requests.");
            std::process::exit(1);
        }));
    }

    cli_config.allow_timeout = matches.get_flag("allow-timeout");

    // Parse failure threshold
    if let Some(threshold_str) = matches.get_one::<String>("failure-threshold") {
        let threshold: f64 = threshold_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Failure threshold '{threshold_str}' is not a valid number. Expected a value between 0-100.");
            std::process::exit(1);
        });
        if !(0.0..=100.0).contains(&threshold) {
            eprintln!(
                "Error: Failure threshold {threshold}% is invalid. Expected a value between 0-100."
            );
            std::process::exit(1);
        }
        cli_config.failure_threshold = Some(threshold);
    }

    // Output & format
    cli_config.quiet = matches.get_flag("quiet");
    cli_config.verbose = matches.get_flag("verbose");
    cli_config.no_progress = matches.get_flag("no-progress");

    if let Some(format_str) = matches.get_one::<String>("format") {
        cli_config.output_format = Some(format_str.clone());
    }

    // Network & security
    if let Some(user_agent_str) = matches.get_one::<String>("user-agent") {
        cli_config.user_agent = Some(user_agent_str.clone());
    }

    if let Some(proxy_str) = matches.get_one::<String>("proxy") {
        cli_config.proxy = Some(proxy_str.clone());
    }

    cli_config.skip_ssl_verification = matches.get_flag("insecure");

    // Configuration
    if let Some(config_file) = matches.get_one::<String>("config") {
        cli_config.config_file = Some(config_file.clone());
    }

    cli_config.no_config = matches.get_flag("no-config");

    // Performance Analysis
    cli_config.show_performance = matches.get_flag("show-performance");

    if let Some(dashboard_path) = matches.get_one::<String>("html-dashboard") {
        cli_config.html_dashboard_path = Some(dashboard_path.clone());
    }

    cli_config
}

/// Convert derive-based CLI arguments directly to CliConfig structure
pub fn cli_to_config(cli: &Cli) -> CliConfig {
    let mut cli_config = CliConfig::default();

    // Core options
    if let Some(timeout) = cli.timeout {
        if timeout == 0 {
            eprintln!(
                "Error: Timeout cannot be 0. Expected a positive integer representing seconds."
            );
            std::process::exit(1);
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
        cli_config.allowed_status_codes = Some(
            status_str
                .split(',')
                .filter_map(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        s.trim()
                            .parse::<u16>()
                            .map_err(|_| {
                                eprintln!(
                                    "Error: Status code '{s}' is not a valid HTTP status code. Expected a number between 100-599."
                                );
                                std::process::exit(1);
                            })
                            .inspect(|&code| {
                                if !(100..=599).contains(&code) {
                                    eprintln!(
                                        "Error: Status code '{code}' is not a valid HTTP status code. Expected a number between 100-599."
                                    );
                                    std::process::exit(1);
                                }
                            })
                            .ok()
                    }
                })
                .collect(),
        );
    }

    if !cli.exclude_pattern.is_empty() {
        cli_config.exclude_patterns = Some(cli.exclude_pattern.clone());
    }

    // Performance & behavior
    if let Some(concurrency) = cli.concurrency {
        if concurrency == 0 {
            eprintln!(
                "Error: Concurrency cannot be 0. Expected a positive integer representing the number of concurrent requests."
            );
            std::process::exit(1);
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

    // Parse failure threshold
    if let Some(threshold) = cli.failure_threshold {
        if !(0.0..=100.0).contains(&threshold) {
            eprintln!(
                "Error: Failure threshold {threshold}% is invalid. Expected a value between 0-100."
            );
            std::process::exit(1);
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

    // Configuration
    cli_config.config_file = cli.config.clone();
    cli_config.no_config = cli.no_config;

    // Performance Analysis
    cli_config.show_performance = cli.show_performance;
    cli_config.html_dashboard_path = cli.html_dashboard.clone();

    cli_config
}

/// Validate CLI arguments using the derive-based CLI structure
pub fn validate_cli_args(cli: &Cli) {
    // Additional validation using the derive-based CLI
    if let Some(timeout) = cli.timeout {
        if timeout == 0 {
            eprintln!(
                "Error: Timeout cannot be 0. Expected a positive integer representing seconds."
            );
            std::process::exit(1);
        }
        if timeout > timeouts::MAX_TIMEOUT_SECONDS {
            eprintln!(
                "Warning: Timeout of {timeout} seconds is quite large. Consider using a smaller value for better user experience."
            );
        }
    }

    if let Some(concurrency) = cli.concurrency {
        if concurrency == 0 {
            eprintln!(
                "Error: Concurrency cannot be 0. Expected a positive integer representing the number of concurrent requests."
            );
            std::process::exit(1);
        }
        if concurrency > 100 {
            eprintln!(
                "Warning: Concurrency of {concurrency} is quite high and may overwhelm servers. Consider using a smaller value."
            );
        }
    }

    // Validate status codes
    if let Some(ref status_str) = cli.allow_status {
        for code_str in status_str.split(',') {
            if let Ok(code) = code_str.trim().parse::<u16>() {
                if !(100..=599).contains(&code) {
                    eprintln!(
                        "Error: Status code '{code}' is not a valid HTTP status code. Expected a number between 100-599."
                    );
                    std::process::exit(1);
                }
            } else if !code_str.trim().is_empty() {
                eprintln!(
                    "Error: Status code '{}' is not a valid number. Expected a number between 100-599.",
                    code_str.trim()
                );
                std::process::exit(1);
            }
        }
    }

    // Validate failure threshold
    if let Some(threshold) = cli.failure_threshold
        && !(0.0..=100.0).contains(&threshold)
    {
        eprintln!(
            "Error: Failure threshold {threshold}% is invalid. Expected a value between 0-100."
        );
        std::process::exit(1);
    }
}

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
            retry: None,
            retry_delay: None,
            rate_limit: None,
            allow_timeout: false,
            failure_threshold: None,
            quiet: false,
            verbose: false,
            format: output_formats::DEFAULT.to_string(),
            no_progress: false,
            user_agent: None,
            proxy: None,
            insecure: false,
            config: None,
            no_config: false,
            show_performance: false,
            html_dashboard: None,
        }
    }

    #[test]
    fn test_cli_to_config_default() {
        let cli = create_default_cli();

        let config = cli_to_config(&cli);

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

        let config = cli_to_config(&cli);

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

        let config = cli_to_config(&cli);

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

        let config = cli_to_config(&cli);

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

        let config = cli_to_config(&cli);

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

        let config = cli_to_config(&cli);

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

        let config = cli_to_config(&cli);
        assert_eq!(config.failure_threshold, Some(100.0));
    }

    #[test]
    fn test_validate_cli_args_valid() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.timeout = Some(30);
        cli.concurrency = Some(4);
        cli.allow_status = Some("200,404".to_string());
        cli.failure_threshold = Some(10.0);

        // Should not panic
        validate_cli_args(&cli);
    }

    #[test]
    fn test_validate_cli_args_high_timeout_warning() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.timeout = Some(3700); // > MAX_TIMEOUT_SECONDS

        // Should not panic, just print warning
        validate_cli_args(&cli);
    }

    #[test]
    fn test_validate_cli_args_high_concurrency_warning() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.concurrency = Some(150); // > 100

        // Should not panic, just print warning
        validate_cli_args(&cli);
    }

    #[test]
    fn test_validate_cli_args_valid_status_codes() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.allow_status = Some("100,200,300,400,500,599".to_string());

        // Should not panic
        validate_cli_args(&cli);
    }

    #[test]
    fn test_validate_cli_args_empty_status_codes() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.allow_status = Some("200, , 404".to_string());

        // Should not panic - empty status codes are ignored
        validate_cli_args(&cli);
    }

    #[test]
    fn test_validate_cli_args_valid_failure_threshold_boundaries() {
        let mut cli = create_default_cli();
        cli.files = vec!["test.md".to_string()];
        cli.failure_threshold = Some(0.0);

        // Should not panic
        validate_cli_args(&cli);

        let mut cli2 = create_default_cli();
        cli2.files = vec!["test.md".to_string()];
        cli2.failure_threshold = Some(100.0);

        // Should not panic
        validate_cli_args(&cli2);
    }

    #[test]
    fn test_parse_cli_args_string_parsing() {
        // Test individual parsing logic for include strings
        let include_str = "md,html,txt";
        let result: Vec<String> = include_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        assert_eq!(
            result,
            vec!["md".to_string(), "html".to_string(), "txt".to_string()]
        );

        // Test allowlist parsing with empty entries
        let allowlist_str = "https://example.com,,https://test.com,";
        let result: Vec<String> = allowlist_str
            .split(',')
            .filter_map(|s| {
                if s.trim().is_empty() {
                    None
                } else {
                    Some(s.trim().to_string())
                }
            })
            .collect();
        assert_eq!(
            result,
            vec![
                "https://example.com".to_string(),
                "https://test.com".to_string()
            ]
        );

        // Test status code parsing with empty entries
        let status_str = "200,,301,302";
        let result: Vec<u16> = status_str
            .split(',')
            .filter_map(|s| {
                if s.trim().is_empty() {
                    None
                } else {
                    s.trim().parse::<u16>().ok()
                }
            })
            .collect();
        assert_eq!(result, vec![200, 301, 302]);
    }
}
