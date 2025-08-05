use clap::{Arg, ArgAction, Command};
use urlsup::config::{CliConfig, Config};
use urlsup::finder::{Finder, UrlFinder};
use urlsup::path_utils::expand_paths;
use urlsup::progress::ProgressReporter;
use urlsup::validator::{ValidateUrls, Validator};

use std::path::Path;

// Core arguments
const ARG_FILES: &str = "FILES";

// Core options
const OPT_RECURSIVE: &str = "recursive";
const OPT_TIMEOUT: &str = "timeout";

// Filtering & inclusion
const OPT_INCLUDE: &str = "include";
const OPT_ALLOWLIST: &str = "allowlist";
const OPT_ALLOW_STATUS: &str = "allow-status";
const OPT_EXCLUDE_PATTERN: &str = "exclude-pattern";

// Performance & behavior
const OPT_CONCURRENCY: &str = "concurrency";
const OPT_RETRY: &str = "retry";
const OPT_RETRY_DELAY: &str = "retry-delay";
const OPT_RATE_LIMIT: &str = "rate-limit";
const OPT_ALLOW_TIMEOUT: &str = "allow-timeout";

// Output & format
const OPT_QUIET: &str = "quiet";
const OPT_VERBOSE: &str = "verbose";
const OPT_FORMAT: &str = "format";
const OPT_NO_PROGRESS: &str = "no-progress";

// Network & security
const OPT_USER_AGENT: &str = "user-agent";
const OPT_PROXY: &str = "proxy";
const OPT_INSECURE: &str = "insecure";

// Configuration
const OPT_CONFIG: &str = "config";
const OPT_NO_CONFIG: &str = "no-config";

// Removed DEFAULT_TIMEOUT as it's now handled by Config::default()

#[tokio::main]
async fn main() {
    // Core arguments
    let files = Arg::new(ARG_FILES)
        .help("Files or directories to check")
        .action(ArgAction::Append)
        .num_args(1)
        .required(true)
        .index(1);

    // Core options
    let recursive = Arg::new(OPT_RECURSIVE)
        .help("Recursively process directories")
        .short('r')
        .long(OPT_RECURSIVE)
        .action(ArgAction::SetTrue);

    let timeout = Arg::new(OPT_TIMEOUT)
        .help("Connection timeout in seconds (default: 30)")
        .short('t')
        .long(OPT_TIMEOUT)
        .value_name("SECONDS")
        .action(ArgAction::Set);

    // Filtering & inclusion
    let include = Arg::new(OPT_INCLUDE)
        .help("File extensions to process (e.g., md,html,txt)")
        .long(OPT_INCLUDE)
        .value_name("EXTENSIONS")
        .action(ArgAction::Set);

    let allowlist = Arg::new(OPT_ALLOWLIST)
        .help("URLs to allow (comma-separated)")
        .long(OPT_ALLOWLIST)
        .value_name("URLS")
        .action(ArgAction::Set);

    let allow_status = Arg::new(OPT_ALLOW_STATUS)
        .help("Status codes to allow (comma-separated)")
        .long(OPT_ALLOW_STATUS)
        .value_name("CODES")
        .action(ArgAction::Set);

    let exclude_pattern = Arg::new(OPT_EXCLUDE_PATTERN)
        .help("URL patterns to exclude (regex)")
        .long(OPT_EXCLUDE_PATTERN)
        .value_name("REGEX")
        .action(ArgAction::Append);

    // Performance & behavior
    let concurrency = Arg::new(OPT_CONCURRENCY)
        .help("Concurrent requests (default: CPU cores)")
        .long(OPT_CONCURRENCY)
        .value_name("COUNT")
        .action(ArgAction::Set);

    let retry = Arg::new(OPT_RETRY)
        .help("Retry attempts for failed requests (default: 0)")
        .long(OPT_RETRY)
        .value_name("COUNT")
        .action(ArgAction::Set);

    let retry_delay = Arg::new(OPT_RETRY_DELAY)
        .help("Delay between retries in ms (default: 1000)")
        .long(OPT_RETRY_DELAY)
        .value_name("MS")
        .action(ArgAction::Set);

    let rate_limit = Arg::new(OPT_RATE_LIMIT)
        .help("Delay between requests in ms (default: 0)")
        .long(OPT_RATE_LIMIT)
        .value_name("MS")
        .action(ArgAction::Set);

    let allow_timeout = Arg::new(OPT_ALLOW_TIMEOUT)
        .help("Allow URLs that timeout")
        .long(OPT_ALLOW_TIMEOUT)
        .action(ArgAction::SetTrue);

    // Output & format
    let quiet = Arg::new(OPT_QUIET)
        .help("Suppress progress output")
        .short('q')
        .long(OPT_QUIET)
        .action(ArgAction::SetTrue);

    let verbose = Arg::new(OPT_VERBOSE)
        .help("Enable verbose logging")
        .short('v')
        .long(OPT_VERBOSE)
        .action(ArgAction::SetTrue);

    let format = Arg::new(OPT_FORMAT)
        .help("Output format")
        .long(OPT_FORMAT)
        .value_name("FORMAT")
        .value_parser(["text", "json"])
        .default_value("text")
        .action(ArgAction::Set);

    let no_progress = Arg::new(OPT_NO_PROGRESS)
        .help("Disable progress bars")
        .long(OPT_NO_PROGRESS)
        .action(ArgAction::SetTrue);

    // Network & security
    let user_agent = Arg::new(OPT_USER_AGENT)
        .help("Custom User-Agent header")
        .long(OPT_USER_AGENT)
        .value_name("AGENT")
        .action(ArgAction::Set);

    let proxy = Arg::new(OPT_PROXY)
        .help("HTTP/HTTPS proxy URL")
        .long(OPT_PROXY)
        .value_name("URL")
        .action(ArgAction::Set);

    let insecure = Arg::new(OPT_INSECURE)
        .help("Skip SSL certificate verification")
        .long(OPT_INSECURE)
        .action(ArgAction::SetTrue);

    // Configuration
    let config = Arg::new(OPT_CONFIG)
        .help("Use specific config file")
        .long(OPT_CONFIG)
        .value_name("FILE")
        .action(ArgAction::Set);

    let no_config = Arg::new(OPT_NO_CONFIG)
        .help("Ignore config files")
        .long(OPT_NO_CONFIG)
        .action(ArgAction::SetTrue);

    let matches = Command::new("urlsup")
        .version("2.0.0")
        .author("Simon Egersand <s.egersand@gmail.com>")
        .about("CLI to validate URLs in files")
        .arg(files)
        // Core options
        .arg(recursive)
        .arg(timeout)
        // Filtering & inclusion
        .arg(include)
        .arg(allowlist)
        .arg(allow_status)
        .arg(exclude_pattern)
        // Performance & behavior
        .arg(concurrency)
        .arg(retry)
        .arg(retry_delay)
        .arg(rate_limit)
        .arg(allow_timeout)
        // Output & format
        .arg(quiet)
        .arg(verbose)
        .arg(format)
        .arg(no_progress)
        // Network & security
        .arg(user_agent)
        .arg(proxy)
        .arg(insecure)
        // Configuration
        .arg(config)
        .arg(no_config)
        .get_matches();

    // Parse CLI arguments into CliConfig
    let cli_config = parse_cli_args(&matches);

    // Load configuration (respecting --no-config and --config flags)
    let mut config = if cli_config.no_config {
        Config::default()
    } else if let Some(ref config_file) = cli_config.config_file {
        Config::load_from_file(config_file).unwrap_or_else(|e| {
            eprintln!("Error loading config file '{config_file}': {e}");
            std::process::exit(1);
        })
    } else {
        Config::load_from_standard_locations()
    };

    // Merge CLI arguments with configuration (CLI takes precedence)
    config.merge_with_cli(&cli_config);

    // Determine output verbosity
    let quiet = cli_config.quiet;
    let verbose = config.verbose.unwrap_or(false);
    let show_progress = !quiet && !cli_config.no_progress;

    // Get files to process
    let files = matches
        .get_many::<String>(ARG_FILES)
        .map(|f| f.map(Path::new).collect::<Vec<&Path>>())
        .unwrap_or_else(|| {
            eprintln!("No files provided");
            std::process::exit(1);
        });

    // Validate input paths exist
    for path in &files {
        if !path.exists() {
            eprintln!(
                "error: invalid value '{}' for '<FILES>...': File not found [\"{}\"]\n\nFor more information, try '--help'.",
                path.display(),
                path.display()
            );
            std::process::exit(2);
        }
    }

    // Get recursive flag
    let recursive = matches.get_flag(OPT_RECURSIVE);

    // Expand directories to file paths using configuration
    let expanded_paths = match expand_paths(files, recursive, config.file_types_as_set().as_ref()) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!("Error expanding paths: {e}");
            std::process::exit(1);
        }
    };

    if expanded_paths.is_empty() {
        eprintln!("No files found to process");
        std::process::exit(1);
    }

    if verbose {
        eprintln!("Found {} files to process", expanded_paths.len());
        if let Some(ref patterns) = config.exclude_patterns {
            eprintln!("Using {} exclude patterns", patterns.len());
        }
    }

    // Find URLs in files
    let finder = Finder::default();
    let file_paths: Vec<&Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

    let url_locations = match finder.find_urls(file_paths) {
        Ok(urls) => urls,
        Err(e) => {
            eprintln!("Error finding URLs: {e}");
            std::process::exit(1);
        }
    };

    let original_url_count = url_locations.len();
    if verbose {
        eprintln!(
            "Found {} URLs in {} files",
            original_url_count,
            expanded_paths.len()
        );
    }

    // Apply exclude patterns if configured
    let filtered_urls = if let Some(ref _patterns) = config.exclude_patterns {
        match config.compile_exclude_patterns() {
            Ok(compiled_patterns) => url_locations
                .into_iter()
                .filter(|url_location| {
                    !compiled_patterns
                        .iter()
                        .any(|pattern| pattern.is_match(&url_location.url))
                })
                .collect(),
            Err(e) => {
                eprintln!("Error compiling exclude patterns: {e}");
                std::process::exit(1);
            }
        }
    } else {
        url_locations
    };

    if verbose && filtered_urls.len() != original_url_count {
        eprintln!(
            "Filtered to {} URLs after applying exclude patterns",
            filtered_urls.len()
        );
    }

    // Initialize progress reporter
    let mut progress = if show_progress {
        Some(ProgressReporter::new(true))
    } else {
        None
    };

    // Validate URLs using new configuration system
    let validator = Validator::default();
    let validation_results = validator
        .validate_urls_with_config(filtered_urls, &config, progress.as_mut())
        .await;

    // Apply filters based on configuration
    let mut filtered_results: Vec<_> = validation_results
        .into_iter()
        .filter(|result| result.is_not_ok())
        .collect();

    // Apply allowlist filtering
    if let Some(ref allowlist) = config.allowlist {
        filtered_results.retain(|result| {
            !allowlist
                .iter()
                .any(|allowed_url| result.url.contains(allowed_url))
        });
    }

    // Apply allowed status codes filtering
    if let Some(ref allowed_codes) = config.allowed_status_codes {
        filtered_results.retain(|result| {
            if let Some(status_code) = result.status_code {
                !allowed_codes.contains(&status_code)
            } else {
                true // Keep non-HTTP errors
            }
        });
    }

    // Apply timeout filtering
    if config.allow_timeout.unwrap_or(false) {
        filtered_results.retain(|result| {
            if let Some(ref description) = result.description {
                description != "operation timed out"
            } else {
                true
            }
        });
    }

    // Output results based on format
    let output_format = config.output_format.as_deref().unwrap_or("text");

    match output_format {
        "json" => {
            // TODO: Implement JSON output format
            if filtered_results.is_empty() {
                println!("{{\"status\": \"success\", \"issues\": []}}");
            } else {
                println!("{{\"status\": \"failure\", \"issues\": [");
                for (i, result) in filtered_results.iter().enumerate() {
                    let comma = if i < filtered_results.len() - 1 {
                        ","
                    } else {
                        ""
                    };
                    println!(
                        "  {{\"url\": \"{}\", \"file\": \"{}\", \"line\": {}, \"status_code\": {}, \"description\": \"{}\"}}{}",
                        result.url,
                        result.file_name,
                        result.line,
                        result
                            .status_code
                            .map(|c| c.to_string())
                            .unwrap_or_else(|| "null".to_string()),
                        result.description.as_deref().unwrap_or(""),
                        comma
                    );
                }
                println!("  ]}}");
            }
        }
        _ => {
            if !quiet {
                if filtered_results.is_empty() {
                    println!("\n✓ No issues found!");
                } else {
                    println!("\n✗ Found {} issues:", filtered_results.len());
                    for (i, result) in filtered_results.iter().enumerate() {
                        println!("{:4}. {}", i + 1, result);
                    }
                }
            }
        }
    }

    // Exit with appropriate code
    if filtered_results.is_empty() {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

fn parse_cli_args(matches: &clap::ArgMatches) -> CliConfig {
    let mut cli_config = CliConfig::default();

    // Core options
    if let Some(timeout_str) = matches.get_one::<String>(OPT_TIMEOUT) {
        cli_config.timeout = Some(timeout_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Could not parse timeout '{timeout_str}' as a valid number");
            std::process::exit(1);
        }));
    }

    // Filtering & inclusion
    if let Some(include_str) = matches.get_one::<String>(OPT_INCLUDE) {
        cli_config.file_types = Some(
            include_str
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        );
    }

    if let Some(allowlist_str) = matches.get_one::<String>(OPT_ALLOWLIST) {
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

    if let Some(status_str) = matches.get_one::<String>(OPT_ALLOW_STATUS) {
        cli_config.allowed_status_codes = Some(
            status_str
                .split(',')
                .filter_map(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        s.trim()
                            .parse()
                            .map_err(|_| {
                                eprintln!(
                                    "Error: Could not parse status code '{s}' as a valid number"
                                );
                                std::process::exit(1);
                            })
                            .ok()
                    }
                })
                .collect(),
        );
    }

    if let Some(patterns) = matches.get_many::<String>(OPT_EXCLUDE_PATTERN) {
        cli_config.exclude_patterns = Some(patterns.cloned().collect());
    }

    // Performance & behavior
    if let Some(concurrency_str) = matches.get_one::<String>(OPT_CONCURRENCY) {
        cli_config.threads = Some(concurrency_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Could not parse concurrency '{concurrency_str}' as a valid number");
            std::process::exit(1);
        }));
    }

    if let Some(retry_str) = matches.get_one::<String>(OPT_RETRY) {
        cli_config.retry_attempts = Some(retry_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Could not parse retry count '{retry_str}' as a valid number");
            std::process::exit(1);
        }));
    }

    if let Some(retry_delay_str) = matches.get_one::<String>(OPT_RETRY_DELAY) {
        cli_config.retry_delay = Some(retry_delay_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Could not parse retry delay '{retry_delay_str}' as a valid number");
            std::process::exit(1);
        }));
    }

    if let Some(rate_limit_str) = matches.get_one::<String>(OPT_RATE_LIMIT) {
        cli_config.rate_limit_delay = Some(rate_limit_str.parse().unwrap_or_else(|_| {
            eprintln!("Error: Could not parse rate limit '{rate_limit_str}' as a valid number");
            std::process::exit(1);
        }));
    }

    cli_config.allow_timeout = matches.get_flag(OPT_ALLOW_TIMEOUT);

    // Output & format
    cli_config.quiet = matches.get_flag(OPT_QUIET);
    cli_config.verbose = matches.get_flag(OPT_VERBOSE);
    cli_config.no_progress = matches.get_flag(OPT_NO_PROGRESS);

    if let Some(format_str) = matches.get_one::<String>(OPT_FORMAT) {
        cli_config.output_format = Some(format_str.clone());
    }

    // Network & security
    if let Some(user_agent_str) = matches.get_one::<String>(OPT_USER_AGENT) {
        cli_config.user_agent = Some(user_agent_str.clone());
    }

    if let Some(proxy_str) = matches.get_one::<String>(OPT_PROXY) {
        cli_config.proxy = Some(proxy_str.clone());
    }

    cli_config.skip_ssl_verification = matches.get_flag(OPT_INSECURE);

    // Configuration
    if let Some(config_file) = matches.get_one::<String>(OPT_CONFIG) {
        cli_config.config_file = Some(config_file.clone());
    }

    cli_config.no_config = matches.get_flag(OPT_NO_CONFIG);

    cli_config
}
