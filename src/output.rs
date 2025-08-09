//! Output formatting and display logic for urlsup

use crate::UrlLocation;
use crate::color::{Colors, colorize};
use crate::config::Config;
use crate::constants::output_formats;
use crate::validator::ValidationResult;

/// Metadata for displaying results
#[derive(Debug, Clone)]
pub struct DisplayMetadata {
    pub total_validated: usize,
    pub issues_found: usize,
    pub files_processed: usize,
    pub total_urls_found: usize,
    pub unique_urls_found: usize,
}

/// Display configuration information in a user-friendly format
pub fn display_config_info(config: &Config, threads: usize, expanded_paths: &[std::path::PathBuf]) {
    println!(
        "{}: {}",
        colorize(
            &format!("{}{}{}", Colors::BOLD, "Using threads", Colors::RESET),
            Colors::BRIGHT_CYAN
        ),
        colorize(&threads.to_string(), Colors::BRIGHT_WHITE)
    );
    println!(
        "{}: {}",
        colorize(
            &format!(
                "{}{}{}",
                Colors::BOLD,
                "Using timeout (seconds)",
                Colors::RESET
            ),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.timeout.unwrap_or(30).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    println!(
        "{}: {}",
        colorize(
            &format!("{}{}{}", Colors::BOLD, "Allow timeout", Colors::RESET),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.allow_timeout.unwrap_or(false).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    println!(
        "{}: {}",
        colorize(
            &format!("{}{}{}", Colors::BOLD, "Retry attempts", Colors::RESET),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.retry_attempts.unwrap_or(0).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    println!(
        "{}: {}",
        colorize(
            &format!("{}{}{}", Colors::BOLD, "Retry delay (ms)", Colors::RESET),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.retry_delay.unwrap_or(1000).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    println!(
        "{}: {}",
        colorize(
            &format!(
                "{}{}{}",
                Colors::BOLD,
                "Rate limit delay (ms)",
                Colors::RESET
            ),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.rate_limit_delay.unwrap_or(0).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    println!(
        "{}: {}",
        colorize(
            &format!("{}{}{}", Colors::BOLD, "Use HEAD requests", Colors::RESET),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.use_head_requests.unwrap_or(false).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    println!(
        "{}: {}",
        colorize(
            &format!(
                "{}{}{}",
                Colors::BOLD,
                "Skip SSL verification",
                Colors::RESET
            ),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &config.skip_ssl_verification.unwrap_or(false).to_string(),
            Colors::BRIGHT_WHITE
        )
    );

    // Show user agent if custom
    if let Some(ref user_agent) = config.user_agent {
        println!(
            "{}: {}",
            colorize(
                &format!("{}{}{}", Colors::BOLD, "User agent", Colors::RESET),
                Colors::BRIGHT_CYAN
            ),
            colorize(user_agent, Colors::BRIGHT_WHITE)
        );
    }

    // Show proxy if configured
    if let Some(ref proxy) = config.proxy {
        println!(
            "{}: {}",
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Proxy", Colors::RESET),
                Colors::BRIGHT_CYAN
            ),
            colorize(proxy, Colors::BRIGHT_WHITE)
        );
    }

    // Show allowlist if configured
    if let Some(ref allowlist) = config.allowlist {
        println!(
            "{}: {}",
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Allowlist", Colors::RESET),
                Colors::BRIGHT_CYAN
            ),
            colorize(&format!("{} URLs", allowlist.len()), Colors::BRIGHT_WHITE)
        );
    }

    // Show allowed status codes if configured
    if let Some(ref codes) = config.allowed_status_codes {
        println!(
            "{}: {}",
            colorize(
                &format!(
                    "{}{}{}",
                    Colors::BOLD,
                    "Allowed status codes",
                    Colors::RESET
                ),
                Colors::BRIGHT_CYAN
            ),
            colorize(&format!("{codes:?}"), Colors::BRIGHT_WHITE)
        );
    }

    println!(
        "\n{} {}: {}",
        colorize("📁", Colors::BRIGHT_BLUE),
        colorize(
            &format!("{}{}{}", Colors::BOLD, "Will check URLs in", Colors::RESET),
            Colors::BRIGHT_CYAN
        ),
        colorize(
            &format!(
                "{}{} file{}{}",
                Colors::BOLD,
                expanded_paths.len(),
                if expanded_paths.len() == 1 { "" } else { "s" },
                Colors::RESET
            ),
            Colors::BRIGHT_WHITE
        )
    );

    // List files (limit to first 10 to avoid spam)
    for (i, path) in expanded_paths.iter().enumerate().take(10) {
        println!(
            "   {}. {}",
            colorize(&format!("{}", i + 1), Colors::DIM),
            colorize(&path.display().to_string(), Colors::BLUE)
        );
    }
    if expanded_paths.len() > 10 {
        println!(
            "   {}",
            colorize(
                &format!("... and {} more files", expanded_paths.len() - 10),
                Colors::DIM
            )
        );
    }
    println!();
}

/// Display URL discovery information
pub fn display_url_discovery(unique_count: usize, total_count: usize, unique_urls: &[UrlLocation]) {
    if unique_count == total_count {
        println!(
            "\n{} {}: {}",
            colorize("🔍", Colors::BRIGHT_GREEN),
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Found", Colors::RESET),
                Colors::BRIGHT_CYAN
            ),
            colorize(
                &format!(
                    "{}{} unique URLs{}",
                    Colors::BOLD,
                    unique_count,
                    Colors::RESET
                ),
                Colors::BRIGHT_WHITE
            )
        );
    } else {
        println!(
            "\n{} {}: {}",
            colorize("🔍", Colors::BRIGHT_GREEN),
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Found", Colors::RESET),
                Colors::BRIGHT_CYAN
            ),
            colorize(
                &format!(
                    "{}{} unique URLs, {} in total{}",
                    Colors::BOLD,
                    unique_count,
                    total_count,
                    Colors::RESET
                ),
                Colors::BRIGHT_WHITE
            )
        );
    }

    // Show all URLs
    for (i, url_location) in unique_urls.iter().enumerate() {
        println!(
            "   {}. {}",
            colorize(&format!("{}", i + 1), Colors::DIM),
            colorize(&url_location.url, Colors::CYAN)
        );
    }
    println!();
}

/// Display validation results based on output format
pub fn display_results(
    filtered_results: &[ValidationResult],
    output_format: &str,
    quiet: bool,
    config: &Config,
    metadata: &DisplayMetadata,
) {
    match output_format {
        output_formats::MINIMAL => display_minimal_output(filtered_results),
        output_formats::JSON => display_json_output(filtered_results, metadata),
        _ => display_text_output(
            filtered_results,
            quiet,
            config,
            metadata.total_validated,
            metadata.issues_found,
        ),
    }
}

/// Display results in minimal format (no colors, emojis, or grouping)
fn display_minimal_output(filtered_results: &[ValidationResult]) {
    for result in filtered_results {
        if let Some(status_code) = result.status_code {
            println!("{} {}", status_code, result.url);
        } else if let Some(ref description) = result.description {
            println!("{} {}", description, result.url);
        } else {
            println!("ERROR {}", result.url);
        }
    }
}

/// Display results in JSON format
fn display_json_output(filtered_results: &[ValidationResult], metadata: &DisplayMetadata) {
    let success_rate = if metadata.total_validated > 0 {
        ((metadata.total_validated - metadata.issues_found) as f64
            / metadata.total_validated as f64)
            * 100.0
    } else {
        100.0
    };

    print!("{{\"files\": {{");
    print!("\"total\": {}, ", metadata.files_processed);
    print!("\"processed\": {}", metadata.files_processed);
    print!("}}, ");

    print!("\"urls\": {{");
    print!("\"total_found\": {}, ", metadata.total_urls_found);
    print!("\"unique\": {}, ", metadata.unique_urls_found);
    print!("\"validated\": {}, ", metadata.total_validated);
    print!("\"failed\": {}, ", metadata.issues_found);
    print!("\"success_rate\": {success_rate:.1}");
    print!("}}, ");

    if filtered_results.is_empty() {
        println!("\"status\": \"success\", \"issues\": []}}");
    } else {
        println!("\"status\": \"failure\", \"issues\": [");
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

/// Display results in text format with colors, emojis, and grouping
fn display_text_output(
    filtered_results: &[ValidationResult],
    quiet: bool,
    config: &Config,
    total_validated: usize,
    issues_found: usize,
) {
    if !quiet {
        if filtered_results.is_empty() {
            println!(
                "{} {}!",
                colorize("✅", Colors::BRIGHT_GREEN),
                colorize(
                    &format!("{}{}{}", Colors::BOLD, "No issues found", Colors::RESET),
                    Colors::BRIGHT_GREEN
                )
            );
        } else {
            println!(
                "{} {}",
                colorize("⚠️", Colors::BRIGHT_RED),
                colorize(
                    &format!("{}{}{}", Colors::BOLD, "Issues", Colors::RESET),
                    Colors::BRIGHT_RED
                )
            );

            display_grouped_issues(filtered_results);
        }
    }

    // Display failure threshold information if configured
    display_failure_threshold_info(config, total_validated, issues_found, quiet);
}

/// Display issues grouped by error type
fn display_grouped_issues(filtered_results: &[ValidationResult]) {
    // Group results by status type
    let mut client_errors = Vec::new(); // 4xx
    let mut server_errors = Vec::new(); // 5xx
    let mut redirects = Vec::new(); // 3xx
    let mut other_http = Vec::new(); // Other HTTP codes
    let mut network_errors = Vec::new(); // No status code

    for result in filtered_results {
        if let Some(status_code) = result.status_code {
            match status_code {
                300..=399 => redirects.push(result),
                400..=499 => client_errors.push(result),
                500..=599 => server_errors.push(result),
                _ => other_http.push(result),
            }
        } else {
            network_errors.push(result);
        }
    }

    // Display network/connection errors first
    if !network_errors.is_empty() {
        println!(
            "\n   {} {}:",
            colorize("🔌", Colors::BRIGHT_YELLOW),
            colorize(
                &format!(
                    "{}{}{}",
                    Colors::BOLD,
                    "Network/Connection Errors",
                    Colors::RESET
                ),
                Colors::BRIGHT_YELLOW
            )
        );
        for (i, result) in network_errors.iter().enumerate() {
            let description = result.description.as_deref().unwrap_or("Unknown error");
            println!(
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(description, Colors::BRIGHT_YELLOW),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display client errors (4xx)
    if !client_errors.is_empty() {
        println!(
            "\n   {} {}:",
            colorize("🚫", Colors::BRIGHT_RED),
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Client Errors (4xx)", Colors::RESET),
                Colors::BRIGHT_RED
            )
        );
        for (i, result) in client_errors.iter().enumerate() {
            let status_code = result.status_code.unwrap();
            println!(
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::BRIGHT_RED),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display server errors (5xx)
    if !server_errors.is_empty() {
        println!(
            "\n   {} {}:",
            colorize("💥", Colors::BRIGHT_MAGENTA),
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Server Errors (5xx)", Colors::RESET),
                Colors::BRIGHT_MAGENTA
            )
        );
        for (i, result) in server_errors.iter().enumerate() {
            let status_code = result.status_code.unwrap();
            println!(
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::BRIGHT_MAGENTA),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display redirect issues (3xx) - if any are flagged as issues
    if !redirects.is_empty() {
        println!(
            "\n   {} {}:",
            colorize("🔄", Colors::BRIGHT_YELLOW),
            colorize(
                &format!(
                    "{}{}{}",
                    Colors::BOLD,
                    "Redirect Issues (3xx)",
                    Colors::RESET
                ),
                Colors::BRIGHT_YELLOW
            )
        );
        for (i, result) in redirects.iter().enumerate() {
            let status_code = result.status_code.unwrap();
            println!(
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::BRIGHT_YELLOW),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display other HTTP issues
    if !other_http.is_empty() {
        println!(
            "\n   {} {}:",
            colorize("❓", Colors::WHITE),
            colorize(
                &format!("{}{}{}", Colors::BOLD, "Other HTTP Issues", Colors::RESET),
                Colors::WHITE
            )
        );
        for (i, result) in other_http.iter().enumerate() {
            let status_code = result.status_code.unwrap();
            println!(
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::WHITE),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }
}

/// Display failure threshold information if configured
fn display_failure_threshold_info(
    config: &Config,
    total_validated: usize,
    issues_found: usize,
    quiet: bool,
) {
    if let Some(threshold) = config.failure_threshold {
        let failure_rate = (issues_found as f64 / total_validated as f64) * 100.0;

        if !quiet {
            if failure_rate > threshold {
                println!(
                    "\n{} Failure rate {:.1}% exceeds threshold {:.1}% ({}/{} URLs failed)",
                    colorize("❌", Colors::BRIGHT_RED),
                    failure_rate,
                    threshold,
                    issues_found,
                    total_validated
                );
            } else if issues_found > 0 {
                println!(
                    "\n{} Failure rate {:.1}% is within threshold {:.1}% ({}/{} URLs failed)",
                    colorize("✅", Colors::BRIGHT_GREEN),
                    failure_rate,
                    threshold,
                    issues_found,
                    total_validated
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::constants::output_formats;
    use crate::types::UrlLocation;
    use crate::validator::ValidationResult;
    use std::io::{self, Write};
    use std::path::PathBuf;

    // Helper function to capture stdout during tests
    #[allow(dead_code)] // Test utility function
    fn capture_output<F, R>(f: F) -> (R, String)
    where
        F: FnOnce() -> R,
    {
        use std::sync::{Arc, Mutex};

        struct TestWriter {
            buffer: Arc<Mutex<Vec<u8>>>,
        }

        impl Write for TestWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.buffer.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        // For now, just run the function and return empty string
        // In a real implementation, we'd need to capture stdout properly
        let result = f();
        (result, String::new())
    }

    #[test]
    fn test_display_config_info_basic() {
        let config = Config::default();
        let paths = vec![PathBuf::from("test.md")];

        // Test doesn't panic and runs successfully
        display_config_info(&config, 4, &paths);
    }

    #[test]
    fn test_display_config_info_with_all_options() {
        let config = Config {
            timeout: Some(60),
            allow_timeout: Some(true),
            retry_attempts: Some(3),
            retry_delay: Some(2000),
            rate_limit_delay: Some(100),
            use_head_requests: Some(true),
            skip_ssl_verification: Some(true),
            user_agent: Some("Custom Agent".to_string()),
            proxy: Some("http://proxy:8080".to_string()),
            allowlist: Some(vec!["example.com".to_string()]),
            allowed_status_codes: Some(vec![200, 404]),
            ..Default::default()
        };

        let paths = vec![PathBuf::from("file1.md"), PathBuf::from("file2.txt")];

        display_config_info(&config, 8, &paths);
    }

    #[test]
    fn test_display_config_info_many_files() {
        let config = Config::default();
        let paths: Vec<PathBuf> = (1..=15)
            .map(|i| PathBuf::from(format!("file{i}.md")))
            .collect();

        display_config_info(&config, 4, &paths);
    }

    #[test]
    fn test_display_config_info_single_file() {
        let config = Config::default();
        let paths = vec![PathBuf::from("single.md")];

        display_config_info(&config, 2, &paths);
    }

    #[test]
    fn test_display_url_discovery_same_count() {
        let url_locations = vec![UrlLocation {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
        }];

        display_url_discovery(1, 1, &url_locations);
    }

    #[test]
    fn test_display_url_discovery_different_count() {
        let url_locations = vec![UrlLocation {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
        }];

        display_url_discovery(1, 3, &url_locations);
    }

    #[test]
    fn test_display_url_discovery_multiple_urls() {
        let url_locations = vec![
            UrlLocation {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
            },
            UrlLocation {
                url: "https://google.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
            },
        ];

        display_url_discovery(2, 5, &url_locations);
    }

    #[test]
    fn test_display_url_discovery_empty() {
        let url_locations = vec![];
        display_url_discovery(0, 0, &url_locations);
    }

    #[test]
    fn test_display_results_minimal() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: None,
        }];
        let config = Config::default();

        let metadata = DisplayMetadata {
            total_validated: 1,
            issues_found: 1,
            files_processed: 1,
            total_urls_found: 1,
            unique_urls_found: 1,
        };
        display_results(&results, output_formats::MINIMAL, false, &config, &metadata);
    }

    #[test]
    fn test_display_results_json() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];
        let config = Config::default();

        let metadata = DisplayMetadata {
            total_validated: 1,
            issues_found: 1,
            files_processed: 1,
            total_urls_found: 1,
            unique_urls_found: 1,
        };
        display_results(&results, output_formats::JSON, false, &config, &metadata);
    }

    #[test]
    fn test_display_results_text() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];
        let config = Config::default();

        let metadata = DisplayMetadata {
            total_validated: 1,
            issues_found: 1,
            files_processed: 1,
            total_urls_found: 1,
            unique_urls_found: 1,
        };
        display_results(&results, output_formats::TEXT, false, &config, &metadata);
    }

    #[test]
    fn test_display_minimal_output_with_status_code() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: None,
        }];

        display_minimal_output(&results);
    }

    #[test]
    fn test_display_minimal_output_with_description() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: None,
            description: Some("Connection timeout".to_string()),
        }];

        display_minimal_output(&results);
    }

    #[test]
    fn test_display_minimal_output_with_neither() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: None,
            description: None,
        }];

        display_minimal_output(&results);
    }

    #[test]
    fn test_display_minimal_output_empty() {
        let results = vec![];
        display_minimal_output(&results);
    }

    #[test]
    fn test_display_json_output_empty() {
        let results = vec![];
        let metadata = DisplayMetadata {
            total_validated: 0,
            issues_found: 0,
            files_processed: 1,
            total_urls_found: 0,
            unique_urls_found: 0,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_json_output_single() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];

        let metadata = DisplayMetadata {
            total_validated: 1,
            issues_found: 1,
            files_processed: 1,
            total_urls_found: 1,
            unique_urls_found: 1,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_json_output_multiple() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://google.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: None,
                description: Some("Connection failed".to_string()),
            },
        ];

        let metadata = DisplayMetadata {
            total_validated: 2,
            issues_found: 2,
            files_processed: 1,
            total_urls_found: 2,
            unique_urls_found: 2,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_json_output_null_values() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: None,
            description: None,
        }];

        let metadata = DisplayMetadata {
            total_validated: 1,
            issues_found: 1,
            files_processed: 1,
            total_urls_found: 1,
            unique_urls_found: 1,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_json_output_metadata_with_success() {
        let results = vec![];
        let metadata = DisplayMetadata {
            total_validated: 5,
            issues_found: 0,
            files_processed: 3,
            total_urls_found: 8,
            unique_urls_found: 5,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_json_output_metadata_with_partial_failures() {
        let results = vec![ValidationResult {
            url: "https://broken.example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];

        let metadata = DisplayMetadata {
            total_validated: 10,
            issues_found: 1,
            files_processed: 2,
            total_urls_found: 12,
            unique_urls_found: 10,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_json_output_large_dataset() {
        let results = vec![];
        let metadata = DisplayMetadata {
            total_validated: 1000,
            issues_found: 0,
            files_processed: 50,
            total_urls_found: 1500,
            unique_urls_found: 1000,
        };
        display_json_output(&results, &metadata);
    }

    #[test]
    fn test_display_metadata_properties() {
        let metadata = DisplayMetadata {
            total_validated: 100,
            issues_found: 5,
            files_processed: 10,
            total_urls_found: 120,
            unique_urls_found: 100,
        };

        assert_eq!(metadata.total_validated, 100);
        assert_eq!(metadata.issues_found, 5);
        assert_eq!(metadata.files_processed, 10);
        assert_eq!(metadata.total_urls_found, 120);
        assert_eq!(metadata.unique_urls_found, 100);
    }

    #[test]
    fn test_display_metadata_clone() {
        let metadata = DisplayMetadata {
            total_validated: 50,
            issues_found: 2,
            files_processed: 5,
            total_urls_found: 60,
            unique_urls_found: 50,
        };

        let cloned = metadata.clone();
        assert_eq!(metadata.total_validated, cloned.total_validated);
        assert_eq!(metadata.issues_found, cloned.issues_found);
        assert_eq!(metadata.files_processed, cloned.files_processed);
        assert_eq!(metadata.total_urls_found, cloned.total_urls_found);
        assert_eq!(metadata.unique_urls_found, cloned.unique_urls_found);
    }

    #[test]
    fn test_display_text_output_empty_not_quiet() {
        let results = vec![];
        let config = Config::default();

        display_text_output(&results, false, &config, 10, 0);
    }

    #[test]
    fn test_display_text_output_empty_quiet() {
        let results = vec![];
        let config = Config::default();

        display_text_output(&results, true, &config, 10, 0);
    }

    #[test]
    fn test_display_text_output_with_issues() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];
        let config = Config::default();

        display_text_output(&results, false, &config, 10, 1);
    }

    #[test]
    fn test_display_grouped_issues_client_errors() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: Some(404),
                description: None,
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: Some(403),
                description: None,
            },
        ];

        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_grouped_issues_server_errors() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: Some(500),
                description: None,
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: Some(502),
                description: None,
            },
        ];

        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_grouped_issues_redirects() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: Some(301),
                description: None,
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: Some(302),
                description: None,
            },
        ];

        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_grouped_issues_network_errors() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: None,
                description: Some("Connection timeout".to_string()),
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: None,
                description: None,
            },
        ];

        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_grouped_issues_other_http() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: Some(100),
                description: None,
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: Some(600),
                description: None,
            },
        ];

        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_grouped_issues_mixed() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                file_name: "test.md".to_string(),
                line: 1,
                status_code: Some(404),
                description: None,
            },
            ValidationResult {
                url: "https://test.com".to_string(),
                file_name: "test.md".to_string(),
                line: 2,
                status_code: Some(500),
                description: None,
            },
            ValidationResult {
                url: "https://timeout.com".to_string(),
                file_name: "test.md".to_string(),
                line: 3,
                status_code: None,
                description: Some("timeout".to_string()),
            },
        ];

        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_grouped_issues_empty() {
        let results = vec![];
        display_grouped_issues(&results);
    }

    #[test]
    fn test_display_failure_threshold_info_no_threshold() {
        let config = Config::default();
        display_failure_threshold_info(&config, 100, 10, false);
    }

    #[test]
    fn test_display_failure_threshold_info_exceeds_threshold() {
        let config = Config {
            failure_threshold: Some(5.0),
            ..Default::default()
        };

        display_failure_threshold_info(&config, 100, 10, false); // 10% > 5%
    }

    #[test]
    fn test_display_failure_threshold_info_within_threshold() {
        let config = Config {
            failure_threshold: Some(15.0),
            ..Default::default()
        };

        display_failure_threshold_info(&config, 100, 10, false); // 10% < 15%
    }

    #[test]
    fn test_display_failure_threshold_info_quiet() {
        let config = Config {
            failure_threshold: Some(5.0),
            ..Default::default()
        };

        display_failure_threshold_info(&config, 100, 10, true);
    }

    #[test]
    fn test_display_failure_threshold_info_no_issues() {
        let config = Config {
            failure_threshold: Some(5.0),
            ..Default::default()
        };

        display_failure_threshold_info(&config, 100, 0, false);
    }

    #[test]
    fn test_display_failure_threshold_info_edge_cases() {
        let mut config = Config {
            failure_threshold: Some(0.0),
            ..Default::default()
        };

        display_failure_threshold_info(&config, 1, 1, false); // 100% > 0%

        config.failure_threshold = Some(100.0);
        display_failure_threshold_info(&config, 100, 100, false); // 100% = 100%
    }
}
