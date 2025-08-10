use clap::{CommandFactory, Parser};
use urlsup::config::Config;
use urlsup::core::constants::{error_messages, output_formats};
use urlsup::discovery::path_utils::expand_paths;
use urlsup::discovery::{Finder, UrlFinder};
use urlsup::reporting::PerformanceProfiler;
use urlsup::reporting::logging;
use urlsup::reporting::{DashboardData, HtmlDashboard};
use urlsup::ui::ProgressReporter;
use urlsup::ui::completion::{install_completion, print_completions};
use urlsup::ui::output;
use urlsup::ui::{Cli, Commands, cli_to_config};
use urlsup::validation::{ValidateUrls, Validator};

use std::path::Path;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Handle completion commands first
    if let Some(exit_code) = handle_completion_commands(&cli) {
        std::process::exit(exit_code);
    }

    // Validate that files are provided when not using completions
    if cli.files.is_empty() {
        eprintln!("Error: No files provided");
        eprintln!("\nFor more information, try '--help'.");
        std::process::exit(1);
    }

    // Run the main URL validation logic
    match run_urlsup_logic(&cli).await {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}

/// Handle completion commands and return exit code if a completion command was processed
pub fn handle_completion_commands(cli: &Cli) -> Option<i32> {
    match cli.command {
        Some(Commands::CompletionGenerate { shell }) => {
            let mut app = Cli::command();
            print_completions(shell, &mut app);
            Some(0)
        }
        Some(Commands::CompletionInstall { shell }) => match install_completion(shell) {
            Ok(message) => {
                println!("{message}");
                Some(0)
            }
            Err(e) => {
                eprintln!("Error: {e}");
                Some(1)
            }
        },
        None => None,
    }
}

/// Main URL validation logic extracted from main() for testing
pub async fn run_urlsup_logic(cli: &Cli) -> Result<i32, Box<dyn std::error::Error>> {
    // Parse CLI arguments into CliConfig using the derive-based CLI
    let cli_config = cli_to_config(cli);

    // Load and merge configuration
    let config = load_and_merge_config(&cli_config)?;

    // Initialize performance profiler if requested
    let mut profiler = if config.show_performance.unwrap_or(false) {
        Some(PerformanceProfiler::new())
    } else {
        None
    };

    // Setup logging and output settings
    let output_settings = setup_output_settings(&cli_config, &config);
    logging::init_logger(output_settings.verbose, output_settings.quiet);

    // Process files and expand paths
    let timer = profiler
        .as_mut()
        .map(|p| p.start_operation("file_processing"));
    let expanded_paths = process_and_expand_files(cli, &config)?;
    if let (Some(profiler), Some(timer)) = (profiler.as_mut(), timer) {
        profiler.finish_operation(timer, expanded_paths.len());
    }

    // Display configuration info if needed
    if output_settings.should_show_config_info() {
        display_configuration_info(&config, &expanded_paths);
    }

    // Find and filter URLs
    let timer = profiler
        .as_mut()
        .map(|p| p.start_operation("url_discovery"));
    let filtered_urls = find_and_filter_urls(&expanded_paths, &config)?;
    if let (Some(profiler), Some(timer)) = (profiler.as_mut(), timer) {
        profiler.finish_operation(timer, filtered_urls.len());
    }

    // Display URL discovery info if needed
    if output_settings.should_show_url_info() {
        display_url_discovery_info(&filtered_urls);
    }

    // Initialize progress reporter
    let mut progress = create_progress_reporter(&output_settings);

    // Validate URLs and process results
    let timer = profiler
        .as_mut()
        .map(|p| p.start_operation("url_validation"));
    let validation_results = validate_urls(&filtered_urls, &config, progress.as_mut()).await?;
    let total_validated = validation_results.len();
    if let (Some(profiler), Some(timer)) = (profiler.as_mut(), timer) {
        profiler.finish_operation(timer, total_validated);
    }

    let filtered_results = apply_result_filters(validation_results, &config);

    // Finalize progress reporting
    finalize_progress_reporter(progress);

    // Calculate URL statistics for JSON output
    let unique_urls =
        urlsup::validation::validator::Validator::deduplicate_urls_optimized(&filtered_urls);
    let unique_urls_found = unique_urls.len();
    let total_urls_found = filtered_urls.len();
    let files_processed = expanded_paths.len();

    // Create display metadata
    let metadata = output::DisplayMetadata {
        total_validated,
        issues_found: filtered_results.len(),
        files_processed,
        total_urls_found,
        unique_urls_found,
    };

    // Display final results and determine exit code
    let (_, issues_found) =
        display_final_results(&filtered_results, &output_settings, &config, &metadata);

    // Generate performance report if requested
    let performance_report = if let Some(profiler) = profiler {
        profiler.display_performance_summary();

        Some(profiler.generate_report())
    } else {
        None
    };

    // Generate HTML dashboard if requested
    if let Some(ref dashboard_path) = config.html_dashboard_path {
        let dashboard_data = DashboardData {
            metadata: metadata.clone(),
            results: filtered_results.clone(),
            performance: performance_report,
            config: config.clone(),
            timestamp: chrono::Utc::now()
                .format("%Y-%m-%d %H:%M:%S UTC")
                .to_string(),
        };

        if let Err(e) = HtmlDashboard::generate_dashboard(&dashboard_data, dashboard_path) {
            eprintln!("Warning: Failed to generate HTML dashboard: {}", e);
        } else {
            println!("📊 HTML dashboard generated: {}", dashboard_path);
        }
    }

    Ok(determine_exit_code(issues_found, total_validated, &config))
}

/// Load configuration from file or standard locations and merge with CLI config
pub fn load_and_merge_config(
    cli_config: &urlsup::config::CliConfig,
) -> Result<Config, Box<dyn std::error::Error>> {
    let mut config = if cli_config.no_config {
        Config::default()
    } else if let Some(ref config_file) = cli_config.config_file {
        Config::load_from_file(config_file).inspect_err(|e| {
            logging::log_error(
                &format!("Could not load config file '{config_file}'"),
                Some(e),
            );
        })?
    } else {
        Config::load_from_standard_locations()
    };

    // Merge CLI arguments with configuration (CLI takes precedence)
    config.merge_with_cli(cli_config);
    Ok(config)
}

/// Settings for output formatting and display
pub struct OutputSettings {
    pub quiet: bool,
    pub verbose: bool,
    pub output_format: String,
    pub show_progress: bool,
}

impl OutputSettings {
    pub fn should_show_config_info(&self) -> bool {
        !self.quiet && self.output_format == output_formats::TEXT
    }

    pub fn should_show_url_info(&self) -> bool {
        !self.quiet && self.output_format == output_formats::TEXT
    }
}

/// Setup output settings based on CLI and config
pub fn setup_output_settings(
    cli_config: &urlsup::config::CliConfig,
    config: &Config,
) -> OutputSettings {
    let quiet = cli_config.quiet;
    let verbose = config.verbose.unwrap_or(false);
    let output_format = config
        .output_format
        .as_deref()
        .unwrap_or(output_formats::DEFAULT)
        .to_string();
    let show_progress = !quiet && !cli_config.no_progress;

    OutputSettings {
        quiet,
        verbose,
        output_format,
        show_progress,
    }
}

/// Process and validate file paths, then expand directories
pub fn process_and_expand_files(
    cli: &Cli,
    config: &Config,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    // Get files to process from the derive-based CLI
    let files: Vec<&Path> = cli.files.iter().map(Path::new).collect();

    // Validate input paths exist
    validate_file_paths(&files)?;

    // Expand directories to file paths using configuration
    let expanded_paths = expand_paths(files, cli.recursive, config.file_types_as_set().as_ref())
        .inspect_err(|e| {
            logging::log_error("Could not expand file paths", Some(e));
        })?;

    if expanded_paths.is_empty() {
        let error = "No files found to process";
        logging::log_error(error, None);
        return Err(error.into());
    }

    // Log file processing information
    logging::log_file_info(expanded_paths.len(), &expanded_paths);

    Ok(expanded_paths)
}

/// Validate that all file paths exist
pub fn validate_file_paths(files: &[&Path]) -> Result<(), Box<dyn std::error::Error>> {
    for path in files {
        if !path.exists() {
            let error_msg = format!("File not found: '{}'", path.display());
            logging::log_error(&error_msg, None);
            return Err(error_msg.into());
        }
    }
    Ok(())
}

/// Display configuration information
pub fn display_configuration_info(config: &Config, expanded_paths: &[std::path::PathBuf]) {
    let threads = config.threads.unwrap_or_else(num_cpus::get);
    // Log configuration info
    logging::log_config_info(config, threads);
    // Display configuration using output module
    output::display_config_info(config, threads, expanded_paths);
}

/// Find URLs in files and apply exclude pattern filtering
pub fn find_and_filter_urls(
    expanded_paths: &[std::path::PathBuf],
    config: &Config,
) -> Result<Vec<urlsup::UrlLocation>, Box<dyn std::error::Error>> {
    // Find URLs in files
    let finder = Finder::default();
    let file_paths: Vec<&Path> = expanded_paths.iter().map(|p| p.as_path()).collect();

    let url_locations = finder.find_urls(file_paths).inspect_err(|e| {
        logging::log_error("Could not find URLs in files", Some(e));
    })?;

    // Apply exclude patterns if configured
    let filtered_urls = if let Some(ref _patterns) = config.exclude_patterns {
        let compiled_patterns = config.compile_exclude_patterns().inspect_err(|e| {
            logging::log_error("Could not compile exclude patterns", Some(e));
        })?;

        url_locations
            .into_iter()
            .filter(|url_location| {
                !compiled_patterns
                    .iter()
                    .any(|pattern| pattern.is_match(&url_location.url))
            })
            .collect()
    } else {
        url_locations
    };

    Ok(filtered_urls)
}

/// Display URL discovery information
pub fn display_url_discovery_info(filtered_urls: &[urlsup::UrlLocation]) {
    // Deduplicate for unique count display
    let unique_urls =
        urlsup::validation::validator::Validator::deduplicate_urls_optimized(filtered_urls);
    let unique_count = unique_urls.len();
    let total_count = filtered_urls.len();

    // Log URL discovery information
    logging::log_url_discovery(unique_count, total_count);

    // Display URL discovery using output module
    output::display_url_discovery(unique_count, total_count, &unique_urls);
}

/// Create progress reporter if needed
pub fn create_progress_reporter(output_settings: &OutputSettings) -> Option<ProgressReporter> {
    if output_settings.show_progress && output_settings.output_format == output_formats::TEXT {
        Some(ProgressReporter::new(true))
    } else {
        None
    }
}

/// Validate URLs using the configured validator
pub async fn validate_urls(
    filtered_urls: &[urlsup::UrlLocation],
    config: &Config,
    progress: Option<&mut ProgressReporter>,
) -> Result<Vec<urlsup::ValidationResult>, Box<dyn std::error::Error>> {
    let validator = Validator::default();

    // Log validation start
    logging::log_validation_start(filtered_urls.len());

    let start_time = std::time::Instant::now();
    let validation_results = validator
        .validate_urls_with_config(filtered_urls.to_vec(), config, progress)
        .await;

    // Log validation completion
    let duration = start_time.elapsed();
    logging::log_validation_complete(validation_results.len(), 0, duration.as_millis());

    Ok(validation_results)
}

/// Apply allowlist, status code, and timeout filters to validation results
pub fn apply_result_filters(
    validation_results: Vec<urlsup::ValidationResult>,
    config: &Config,
) -> Vec<urlsup::ValidationResult> {
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
                description != error_messages::OPERATION_TIMED_OUT
            } else {
                true
            }
        });
    }

    filtered_results
}

/// Finalize progress reporting
pub fn finalize_progress_reporter(progress: Option<ProgressReporter>) {
    if let Some(ref progress) = progress {
        progress.finish_and_clear();
    }
}

/// Display final results and return counts
pub fn display_final_results(
    filtered_results: &[urlsup::ValidationResult],
    output_settings: &OutputSettings,
    config: &Config,
    metadata: &output::DisplayMetadata,
) -> (usize, usize) {
    let issues_found = filtered_results.len();

    // Log validation completion with correct counts
    logging::log_validation_complete(metadata.total_validated, issues_found, 0);

    // Output results using the output module
    output::display_results(
        filtered_results,
        &output_settings.output_format,
        output_settings.quiet,
        config,
        metadata,
    );

    (metadata.total_validated, issues_found)
}

/// Determine exit code based on failure threshold
pub fn determine_exit_code(issues_found: usize, total_validated: usize, config: &Config) -> i32 {
    let should_fail = if let Some(threshold) = config.failure_threshold {
        let failure_rate = (issues_found as f64 / total_validated as f64) * 100.0;
        failure_rate > threshold
    } else {
        issues_found > 0 // Default behavior - fail on any issues
    };

    if should_fail { 1 } else { 0 }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)] // Test code for clarity
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use urlsup::config::{CliConfig, Config};
    use urlsup::{UrlLocation, ValidationResult};

    fn create_test_cli() -> Cli {
        Cli {
            command: None,
            files: vec!["test.md".to_string()],
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
            format: "text".to_string(),
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
    fn test_handle_special_commands_none() {
        let cli = create_test_cli();
        let result = handle_special_commands(&cli);
        assert_eq!(result, None);
    }

    #[test]
    fn test_handle_special_commands_generate() {
        // Test completion generation logic without printing to stdout
        // This test validates the same functionality as handle_special_commands for CompletionGenerate
        // but uses a buffer to capture output instead of printing to stdout during tests

        let mut cli = create_test_cli();
        cli.command = Some(Commands::CompletionGenerate {
            shell: clap_complete::Shell::Bash,
        });

        // Test the completion generation directly using a buffer instead of stdout
        let mut app = Cli::command();
        let app_name = app.get_name().to_string();
        let mut buffer = Vec::new();
        clap_complete::generate(clap_complete::shells::Bash, &mut app, app_name, &mut buffer);

        // Verify that completion script was generated
        assert!(!buffer.is_empty(), "Completion script should be generated");
        let completion_content = String::from_utf8(buffer).expect("Valid UTF-8");
        assert!(
            completion_content.contains("urlsup"),
            "Completion should contain app name"
        );

        // Test that the CLI command parsing works correctly for completion generation
        match cli.command {
            Some(Commands::CompletionGenerate { shell }) => {
                assert_eq!(shell, clap_complete::Shell::Bash);
            }
            _ => panic!("Expected CompletionGenerate command"),
        }
    }

    #[test]
    fn test_handle_special_commands_install_bash() {
        let temp_dir = TempDir::new().unwrap();
        let temp_home = temp_dir.path().to_str().unwrap();

        // Save original HOME
        let original_home = std::env::var("HOME").ok();

        // Set temporary HOME
        unsafe {
            std::env::set_var("HOME", temp_home);
        }

        let mut cli = create_test_cli();
        cli.command = Some(Commands::CompletionInstall {
            shell: clap_complete::Shell::Bash,
        });
        let result = handle_special_commands(&cli);
        assert_eq!(result, Some(0));

        // Restore original HOME
        if let Some(home) = original_home {
            unsafe {
                std::env::set_var("HOME", home);
            }
        } else {
            unsafe {
                std::env::remove_var("HOME");
            }
        }
    }

    #[test]
    fn test_handle_special_commands_install_unsupported() {
        let mut cli = create_test_cli();
        cli.command = Some(Commands::CompletionInstall {
            shell: clap_complete::Shell::PowerShell,
        });
        let result = handle_special_commands(&cli);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_load_and_merge_config_default() {
        let cli_config = CliConfig::default();
        let result = load_and_merge_config(&cli_config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_and_merge_config_no_config_flag() {
        let mut cli_config = CliConfig::default();
        cli_config.no_config = true;
        let result = load_and_merge_config(&cli_config);
        assert!(result.is_ok());
        let config = result.unwrap();
        // Should be default config since no_config is true
        assert_eq!(config.timeout, Some(30)); // Default timeout is 30
    }

    #[test]
    fn test_load_and_merge_config_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_content = r#"
            timeout = 45
            threads = 6
        "#;
        fs::write(&config_path, config_content).unwrap();

        let mut cli_config = CliConfig::default();
        cli_config.config_file = Some(config_path.to_str().unwrap().to_string());

        let result = load_and_merge_config(&cli_config);
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.timeout, Some(45));
        assert_eq!(config.threads, Some(6));
    }

    #[test]
    fn test_load_and_merge_config_invalid_file() {
        let mut cli_config = CliConfig::default();
        cli_config.config_file = Some("/nonexistent/config.toml".to_string());

        let result = load_and_merge_config(&cli_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_setup_output_settings_default() {
        let cli_config = CliConfig::default();
        let config = Config::default();
        let settings = setup_output_settings(&cli_config, &config);

        assert!(!settings.quiet);
        assert!(!settings.verbose);
        assert_eq!(settings.output_format, output_formats::DEFAULT.to_string());
        assert!(settings.show_progress);
    }

    #[test]
    fn test_setup_output_settings_quiet() {
        let mut cli_config = CliConfig::default();
        cli_config.quiet = true;
        let config = Config::default();
        let settings = setup_output_settings(&cli_config, &config);

        assert!(settings.quiet);
        assert!(!settings.show_progress);
    }

    #[test]
    fn test_setup_output_settings_no_progress() {
        let mut cli_config = CliConfig::default();
        cli_config.no_progress = true;
        let config = Config::default();
        let settings = setup_output_settings(&cli_config, &config);

        assert!(!settings.show_progress);
    }

    #[test]
    fn test_setup_output_settings_verbose() {
        let cli_config = CliConfig::default();
        let mut config = Config::default();
        config.verbose = Some(true);
        let settings = setup_output_settings(&cli_config, &config);

        assert!(settings.verbose);
    }

    #[test]
    fn test_setup_output_settings_json_format() {
        let cli_config = CliConfig::default();
        let mut config = Config::default();
        config.output_format = Some(output_formats::JSON.to_string());
        let settings = setup_output_settings(&cli_config, &config);

        assert_eq!(settings.output_format, output_formats::JSON.to_string());
    }

    #[test]
    fn test_output_settings_should_show_config_info() {
        let settings = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };
        assert!(settings.should_show_config_info());

        let settings_quiet = OutputSettings {
            quiet: true,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };
        assert!(!settings_quiet.should_show_config_info());

        let settings_json = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: output_formats::JSON.to_string(),
            show_progress: true,
        };
        assert!(!settings_json.should_show_config_info());
    }

    #[test]
    fn test_output_settings_should_show_url_info() {
        let settings = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };
        assert!(settings.should_show_url_info());

        let settings_quiet = OutputSettings {
            quiet: true,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };
        assert!(!settings_quiet.should_show_url_info());

        let settings_minimal = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: output_formats::MINIMAL.to_string(),
            show_progress: true,
        };
        assert!(!settings_minimal.should_show_url_info());
    }

    #[test]
    fn test_validate_file_paths_valid() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "# Test").unwrap();

        let files = vec![test_file.as_path()];
        let result = validate_file_paths(&files);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_file_paths_invalid() {
        let files = vec![Path::new("/nonexistent/file.md")];
        let result = validate_file_paths(&files);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_validate_file_paths_mixed() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "# Test").unwrap();

        let files = vec![test_file.as_path(), Path::new("/nonexistent/file.md")];
        let result = validate_file_paths(&files);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_and_expand_files_valid() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "# Test").unwrap();

        let mut cli = create_test_cli();
        cli.files = vec![test_file.to_str().unwrap().to_string()];

        let config = Config::default();
        let result = process_and_expand_files(&cli, &config);
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], test_file);
    }

    #[test]
    fn test_process_and_expand_files_nonexistent() {
        let mut cli = create_test_cli();
        cli.files = vec!["/nonexistent/file.md".to_string()];

        let config = Config::default();
        let result = process_and_expand_files(&cli, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_and_expand_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let test_file1 = temp_dir.path().join("test1.md");
        let test_file2 = subdir.join("test2.md");
        fs::write(&test_file1, "# Test 1").unwrap();
        fs::write(&test_file2, "# Test 2").unwrap();

        let mut cli = create_test_cli();
        cli.files = vec![temp_dir.path().to_str().unwrap().to_string()];
        cli.recursive = true;

        let mut config = Config::default();
        config.file_types = Some(vec!["md".to_string()]);

        let result = process_and_expand_files(&cli, &config);
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert!(paths.len() >= 2);
    }

    #[test]
    fn test_find_and_filter_urls_basic() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "Visit https://example.com for more info").unwrap();

        let paths = vec![test_file];
        let config = Config::default();

        let result = find_and_filter_urls(&paths, &config);
        assert!(result.is_ok());
        let urls = result.unwrap();
        assert!(!urls.is_empty());
        assert!(urls.iter().any(|url| url.url.contains("example.com")));
    }

    #[test]
    fn test_find_and_filter_urls_with_exclude_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "Visit https://example.com and https://test.com").unwrap();

        let paths = vec![test_file];
        let mut config = Config::default();
        config.exclude_patterns = Some(vec![".*test.*".to_string()]);

        let result = find_and_filter_urls(&paths, &config);
        assert!(result.is_ok());
        let urls = result.unwrap();
        assert!(urls.iter().any(|url| url.url.contains("example.com")));
        assert!(!urls.iter().any(|url| url.url.contains("test.com")));
    }

    #[test]
    fn test_find_and_filter_urls_invalid_regex() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "Visit https://example.com").unwrap();

        let paths = vec![test_file];
        let mut config = Config::default();
        config.exclude_patterns = Some(vec!["[".to_string()]); // Invalid regex

        let result = find_and_filter_urls(&paths, &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_display_url_discovery_info() {
        let url_locations = vec![
            UrlLocation {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
            },
            UrlLocation {
                url: "https://example.com".to_string(), // Duplicate
                file_name: "test2.md".to_string(),
                line: 2,
            },
            UrlLocation {
                url: "https://google.com".to_string(),
                file_name: "test.md".to_string(),
                line: 3,
            },
        ];

        // Should not panic - this tests the display function
        display_url_discovery_info(&url_locations);
    }

    #[test]
    fn test_create_progress_reporter_enabled() {
        let settings = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };

        let progress = create_progress_reporter(&settings);
        assert!(progress.is_some());
    }

    #[test]
    fn test_create_progress_reporter_disabled_quiet() {
        let settings = OutputSettings {
            quiet: true,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: false,
        };

        let progress = create_progress_reporter(&settings);
        assert!(progress.is_none());
    }

    #[test]
    fn test_create_progress_reporter_disabled_json() {
        let settings = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: output_formats::JSON.to_string(),
            show_progress: true,
        };

        let progress = create_progress_reporter(&settings);
        assert!(progress.is_none());
    }

    #[test]
    fn test_apply_result_filters_basic() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://google.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(200),
                description: None,
            },
        ];

        let config = Config::default();
        let filtered = apply_result_filters(results, &config);

        // Should only include non-OK results
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status_code, Some(404));
    }

    #[test]
    fn test_apply_result_filters_with_allowlist() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://blocked.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
        ];

        let mut config = Config::default();
        config.allowlist = Some(vec!["example.com".to_string()]);

        let filtered = apply_result_filters(results, &config);

        // Should exclude allowlisted URLs
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].url.contains("blocked.com"));
    }

    #[test]
    fn test_apply_result_filters_with_allowed_status_codes() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://server-error.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(500),
                description: Some("Server Error".to_string()),
            },
        ];

        let mut config = Config::default();
        config.allowed_status_codes = Some(vec![404]);

        let filtered = apply_result_filters(results, &config);

        // Should exclude allowed status codes
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status_code, Some(500));
    }

    #[test]
    fn test_apply_result_filters_with_timeout_allowed() {
        let results = vec![
            ValidationResult {
                url: "https://timeout.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some(error_messages::OPERATION_TIMED_OUT.to_string()),
            },
            ValidationResult {
                url: "https://error.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
        ];

        let mut config = Config::default();
        config.allow_timeout = Some(true);

        let filtered = apply_result_filters(results, &config);

        // Should exclude timeout errors
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].status_code, Some(404));
    }

    #[test]
    fn test_finalize_progress_reporter_some() {
        let progress_settings = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };

        let progress = create_progress_reporter(&progress_settings);

        // Should not panic
        finalize_progress_reporter(progress);
    }

    #[test]
    fn test_finalize_progress_reporter_none() {
        // Should not panic
        finalize_progress_reporter(None);
    }

    #[test]
    fn test_display_final_results() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];

        let settings = OutputSettings {
            quiet: false,
            verbose: false,
            output_format: "text".to_string(),
            show_progress: true,
        };

        let config = Config::default();

        let metadata = output::DisplayMetadata {
            total_validated: 1,
            issues_found: 1,
            files_processed: 1,
            total_urls_found: 1,
            unique_urls_found: 1,
        };
        let (total, issues) = display_final_results(&results, &settings, &config, &metadata);
        assert_eq!(total, 1);
        assert_eq!(issues, 1);
    }

    #[test]
    fn test_determine_exit_code_no_issues() {
        let config = Config::default();
        let exit_code = determine_exit_code(0, 10, &config);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_determine_exit_code_with_issues() {
        let config = Config::default();
        let exit_code = determine_exit_code(3, 10, &config);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_determine_exit_code_with_threshold_below() {
        let mut config = Config::default();
        config.failure_threshold = Some(50.0); // 50%

        // 20% failure rate (2 out of 10) - should pass
        let exit_code = determine_exit_code(2, 10, &config);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_determine_exit_code_with_threshold_above() {
        let mut config = Config::default();
        config.failure_threshold = Some(50.0); // 50%

        // 70% failure rate (7 out of 10) - should fail
        let exit_code = determine_exit_code(7, 10, &config);
        assert_eq!(exit_code, 1);
    }

    #[test]
    fn test_determine_exit_code_with_threshold_exact() {
        let mut config = Config::default();
        config.failure_threshold = Some(50.0); // 50%

        // Exactly 50% failure rate (5 out of 10) - should pass (not greater than)
        let exit_code = determine_exit_code(5, 10, &config);
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_determine_exit_code_zero_total() {
        let mut config = Config::default();
        config.failure_threshold = Some(50.0);

        // Edge case: 0 total URLs
        let exit_code = determine_exit_code(0, 0, &config);
        // This would result in NaN, but 0 issues should pass
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_display_configuration_info() {
        let config = Config::default();
        let paths = vec![std::path::PathBuf::from("test.md")];

        // Should not panic
        display_configuration_info(&config, &paths);
    }

    #[test]
    fn test_output_settings_edge_cases() {
        let cli_config = CliConfig::default();
        let config = Config::default();

        // Test different combinations
        let settings1 = setup_output_settings(&cli_config, &config);
        assert!(!settings1.quiet);
        assert!(!settings1.verbose);

        // Test with quiet and verbose both set (quiet should win)
        let mut cli_config_mixed = CliConfig::default();
        cli_config_mixed.quiet = true;
        cli_config_mixed.verbose = true;
        let settings2 = setup_output_settings(&cli_config_mixed, &config);
        assert!(settings2.quiet);
        assert!(!settings2.verbose); // verbose gets overridden by quiet
    }

    #[test]
    fn test_load_and_merge_config_edge_cases() {
        // Test with no_config = true
        let mut cli_config = CliConfig::default();
        cli_config.no_config = true;

        let result = load_and_merge_config(&cli_config);
        assert!(result.is_ok());

        // Should use default config when no_config is true
        let config = result.unwrap();
        assert_eq!(config.timeout, Config::default().timeout);
    }
}
