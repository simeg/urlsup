//! Output formatting and display logic for urlsup

use crate::UrlLocation;
use crate::config::Config;
use crate::core::constants::{output_formats, timeouts};
use crate::ui::color::{Colors, bold, colorize};
use crate::validation::validator::ValidationResult;
use std::fmt::Write as _;

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
pub fn render_config_info(
    config: &Config,
    threads: usize,
    expanded_paths: &[std::path::PathBuf],
) -> String {
    let mut out = String::new();

    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Using threads"), Colors::BRIGHT_CYAN),
        colorize(&threads.to_string(), Colors::BRIGHT_WHITE)
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Using timeout (seconds)"), Colors::BRIGHT_CYAN),
        colorize(
            &config
                .timeout
                .unwrap_or(timeouts::DEFAULT_TIMEOUT_SECONDS)
                .to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Allow timeout"), Colors::BRIGHT_CYAN),
        colorize(
            &config.allow_timeout.unwrap_or(false).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Retry attempts"), Colors::BRIGHT_CYAN),
        colorize(
            &config.retry_attempts.unwrap_or(0).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Retry delay (ms)"), Colors::BRIGHT_CYAN),
        colorize(
            &config.retry_delay.unwrap_or(1000).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Rate limit delay (ms)"), Colors::BRIGHT_CYAN),
        colorize(
            &config.rate_limit_delay.unwrap_or(0).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Use HEAD requests"), Colors::BRIGHT_CYAN),
        colorize(
            &config.use_head_requests.unwrap_or(false).to_string(),
            Colors::BRIGHT_WHITE
        )
    );
    let _ = writeln!(
        out,
        "{}: {}",
        colorize(&bold("Skip SSL verification"), Colors::BRIGHT_CYAN),
        colorize(
            &config.skip_ssl_verification.unwrap_or(false).to_string(),
            Colors::BRIGHT_WHITE
        )
    );

    // Show user agent if custom
    if let Some(ref user_agent) = config.user_agent {
        let _ = writeln!(
            out,
            "{}: {}",
            colorize(&bold("User agent"), Colors::BRIGHT_CYAN),
            colorize(user_agent, Colors::BRIGHT_WHITE)
        );
    }

    // Show proxy if configured
    if let Some(ref proxy) = config.proxy {
        let _ = writeln!(
            out,
            "{}: {}",
            colorize(&bold("Proxy"), Colors::BRIGHT_CYAN),
            colorize(proxy, Colors::BRIGHT_WHITE)
        );
    }

    // Show allowlist if configured
    if let Some(ref allowlist) = config.allowlist {
        let _ = writeln!(
            out,
            "{}: {}",
            colorize(&bold("Allowlist"), Colors::BRIGHT_CYAN),
            colorize(&format!("{} URLs", allowlist.len()), Colors::BRIGHT_WHITE)
        );
    }

    // Show allowed status codes if configured
    if let Some(ref codes) = config.allowed_status_codes {
        let _ = writeln!(
            out,
            "{}: {}",
            colorize(&bold("Allowed status codes"), Colors::BRIGHT_CYAN),
            colorize(&format!("{codes:?}"), Colors::BRIGHT_WHITE)
        );
    }

    let _ = writeln!(
        out,
        "\n{} {}: {}",
        colorize("📁", Colors::BRIGHT_BLUE),
        colorize(&bold("Will check URLs in"), Colors::BRIGHT_CYAN),
        colorize(
            &bold(&format!(
                "{} file{}",
                expanded_paths.len(),
                if expanded_paths.len() == 1 { "" } else { "s" }
            )),
            Colors::BRIGHT_WHITE
        )
    );

    // List files (limit to first 10 to avoid spam)
    for (i, path) in expanded_paths.iter().enumerate().take(10) {
        let _ = writeln!(
            out,
            "   {}. {}",
            colorize(&format!("{}", i + 1), Colors::DIM),
            colorize(&path.display().to_string(), Colors::BLUE)
        );
    }
    if expanded_paths.len() > 10 {
        let _ = writeln!(
            out,
            "   {}",
            colorize(
                &format!("... and {} more files", expanded_paths.len() - 10),
                Colors::DIM
            )
        );
    }
    out.push('\n');

    out
}

/// Display configuration information in a user-friendly format
pub fn display_config_info(config: &Config, threads: usize, expanded_paths: &[std::path::PathBuf]) {
    print!("{}", render_config_info(config, threads, expanded_paths));
}

/// Display URL discovery information
pub fn render_url_discovery(
    unique_count: usize,
    total_count: usize,
    unique_urls: &[UrlLocation],
) -> String {
    let mut out = String::new();

    if unique_count == total_count {
        let _ = writeln!(
            out,
            "\n{} {}: {}",
            colorize("🔍", Colors::BRIGHT_GREEN),
            colorize(&bold("Found"), Colors::BRIGHT_CYAN),
            colorize(
                &bold(&format!("{unique_count} unique URLs")),
                Colors::BRIGHT_WHITE
            )
        );
    } else {
        let _ = writeln!(
            out,
            "\n{} {}: {}",
            colorize("🔍", Colors::BRIGHT_GREEN),
            colorize(&bold("Found"), Colors::BRIGHT_CYAN),
            colorize(
                &bold(&format!(
                    "{unique_count} unique URLs, {total_count} in total"
                )),
                Colors::BRIGHT_WHITE
            )
        );
    }

    // Show all URLs
    for (i, url_location) in unique_urls.iter().enumerate() {
        let _ = writeln!(
            out,
            "   {}. {}",
            colorize(&format!("{}", i + 1), Colors::DIM),
            colorize(&url_location.url, Colors::CYAN)
        );
    }
    out.push('\n');

    out
}

/// Display URL discovery information
pub fn display_url_discovery(unique_count: usize, total_count: usize, unique_urls: &[UrlLocation]) {
    print!(
        "{}",
        render_url_discovery(unique_count, total_count, unique_urls)
    );
}

/// Display validation results based on output format
pub fn render_results(
    filtered_results: &[ValidationResult],
    output_format: &str,
    quiet: bool,
    config: &Config,
    metadata: &DisplayMetadata,
) -> String {
    match output_format {
        output_formats::MINIMAL => render_minimal_output(filtered_results),
        output_formats::JSON => match render_json_output(filtered_results, metadata) {
            Ok(json) => format!("{json}\n"),
            Err(e) => {
                eprintln!("Error: failed to serialize JSON output: {e}");
                String::new()
            }
        },
        _ => render_text_output(
            filtered_results,
            quiet,
            config,
            metadata.total_validated,
            metadata.issues_found,
        ),
    }
}

/// Display validation results based on output format
pub fn display_results(
    filtered_results: &[ValidationResult],
    output_format: &str,
    quiet: bool,
    config: &Config,
    metadata: &DisplayMetadata,
) {
    print!(
        "{}",
        render_results(filtered_results, output_format, quiet, config, metadata)
    );
}

/// Render results in minimal format (no colors, emojis, or grouping).
///
/// Split from printing so the output can be asserted on directly. The
/// `display_*` functions previously wrote to stdout via `println!` with no
/// injectable writer, which is why dozens of tests could only call them and
/// drop the result.
fn render_minimal_output(filtered_results: &[ValidationResult]) -> String {
    let mut out = String::new();
    for result in filtered_results {
        if let Some(status_code) = result.status_code {
            let _ = writeln!(out, "{} {}", status_code, result.url);
        } else if let Some(ref description) = result.description {
            let _ = writeln!(out, "{} {}", description, result.url);
        } else {
            let _ = writeln!(out, "ERROR {}", result.url);
        }
    }
    out
}

/// JSON report schema. Kept separate from the domain types so the public
/// output format does not drift when internal structs change.
#[derive(serde::Serialize)]
struct JsonReport {
    files: JsonFiles,
    urls: JsonUrls,
    status: &'static str,
    issues: Vec<JsonIssue>,
}

#[derive(serde::Serialize)]
struct JsonFiles {
    total: usize,
    processed: usize,
}

#[derive(serde::Serialize)]
struct JsonUrls {
    total_found: usize,
    unique: usize,
    validated: usize,
    failed: usize,
    success_rate: f64,
}

#[derive(serde::Serialize)]
struct JsonIssue {
    url: String,
    file: String,
    line: u64,
    /// `null` when the request never produced a status. This matches the
    /// pre-serde output, which emitted a bare `null` here.
    status_code: Option<u16>,
    /// Always a string -- the pre-serde output used `.unwrap_or("")`, so
    /// consumers may do string operations on this without a null check.
    description: String,
}

/// Render results as a JSON string.
///
/// Split from printing so it can be asserted on directly; the previous
/// hand-rolled `print!` version emitted invalid JSON for any URL, filename or
/// error message containing a quote or backslash.
fn render_json_output(
    filtered_results: &[ValidationResult],
    metadata: &DisplayMetadata,
) -> Result<String, serde_json::Error> {
    let success_rate = if metadata.total_validated > 0 {
        ((metadata.total_validated - metadata.issues_found) as f64
            / metadata.total_validated as f64)
            * 100.0
    } else {
        100.0
    };

    let report = JsonReport {
        files: JsonFiles {
            total: metadata.files_processed,
            processed: metadata.files_processed,
        },
        urls: JsonUrls {
            total_found: metadata.total_urls_found,
            unique: metadata.unique_urls_found,
            validated: metadata.total_validated,
            failed: metadata.issues_found,
            success_rate: (success_rate * 10.0).round() / 10.0,
        },
        status: if filtered_results.is_empty() {
            "success"
        } else {
            "failure"
        },
        issues: filtered_results
            .iter()
            .map(|r| JsonIssue {
                url: r.url.clone(),
                file: r.file_name.clone(),
                line: r.line,
                status_code: r.status_code,
                description: r.description.clone().unwrap_or_default(),
            })
            .collect(),
    };

    // Single-line, matching the pre-serde output. Pretty-printing would break
    // consumers doing line-oriented reads (grep/head) on this format.
    serde_json::to_string(&report)
}

/// Render results in text format with colors, emojis, and grouping.
fn render_text_output(
    filtered_results: &[ValidationResult],
    quiet: bool,
    config: &Config,
    total_validated: usize,
    issues_found: usize,
) -> String {
    let mut out = String::new();

    if !quiet {
        if filtered_results.is_empty() {
            let _ = writeln!(
                out,
                "{} {}!",
                colorize("✅", Colors::BRIGHT_GREEN),
                colorize(&bold("No issues found"), Colors::BRIGHT_GREEN)
            );
        } else {
            let _ = writeln!(
                out,
                "{} {}",
                colorize("⚠️", Colors::BRIGHT_RED),
                colorize(&bold("Issues"), Colors::BRIGHT_RED)
            );

            out.push_str(&render_grouped_issues(filtered_results));
        }
    }

    // Display failure threshold information if configured
    out.push_str(&render_failure_threshold_info(
        config,
        total_validated,
        issues_found,
        quiet,
    ));

    out
}

/// Render issues grouped by error type.
fn render_grouped_issues(filtered_results: &[ValidationResult]) -> String {
    let mut out = String::new();

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
        let _ = writeln!(
            out,
            "\n   {} {}:",
            colorize("🔌", Colors::BRIGHT_YELLOW),
            colorize(&bold("Network/Connection Errors"), Colors::BRIGHT_YELLOW)
        );
        for (i, result) in network_errors.iter().enumerate() {
            let description = result.description.as_deref().unwrap_or("Unknown error");
            let _ = writeln!(
                out,
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(description, Colors::BRIGHT_YELLOW),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display client errors (4xx)
    if !client_errors.is_empty() {
        let _ = writeln!(
            out,
            "\n   {} {}:",
            colorize("🚫", Colors::BRIGHT_RED),
            colorize(&bold("Client Errors (4xx)"), Colors::BRIGHT_RED)
        );
        for (i, result) in client_errors.iter().enumerate() {
            // Safe: this bucket is only populated from the `Some` branch above.
            let status_code = result.status_code.unwrap_or_default();
            let _ = writeln!(
                out,
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::BRIGHT_RED),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display server errors (5xx)
    if !server_errors.is_empty() {
        let _ = writeln!(
            out,
            "\n   {} {}:",
            colorize("💥", Colors::BRIGHT_MAGENTA),
            colorize(&bold("Server Errors (5xx)"), Colors::BRIGHT_MAGENTA)
        );
        for (i, result) in server_errors.iter().enumerate() {
            // Safe: this bucket is only populated from the `Some` branch above.
            let status_code = result.status_code.unwrap_or_default();
            let _ = writeln!(
                out,
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::BRIGHT_MAGENTA),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display redirect issues (3xx) - if any are flagged as issues
    if !redirects.is_empty() {
        let _ = writeln!(
            out,
            "\n   {} {}:",
            colorize("🔄", Colors::BRIGHT_YELLOW),
            colorize(&bold("Redirect Issues (3xx)"), Colors::BRIGHT_YELLOW)
        );
        for (i, result) in redirects.iter().enumerate() {
            // Safe: this bucket is only populated from the `Some` branch above.
            let status_code = result.status_code.unwrap_or_default();
            let _ = writeln!(
                out,
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::BRIGHT_YELLOW),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    // Display other HTTP issues
    if !other_http.is_empty() {
        let _ = writeln!(
            out,
            "\n   {} {}:",
            colorize("❓", Colors::WHITE),
            colorize(&bold("Other HTTP Issues"), Colors::WHITE)
        );
        for (i, result) in other_http.iter().enumerate() {
            // Safe: this bucket is only populated from the `Some` branch above.
            let status_code = result.status_code.unwrap_or_default();
            let _ = writeln!(
                out,
                "      {}. {} {}",
                colorize(&format!("{}", i + 1), Colors::DIM),
                colorize(&status_code.to_string(), Colors::WHITE),
                colorize(&result.url, Colors::CYAN)
            );
        }
    }

    out
}

/// Render failure threshold information if configured.
fn render_failure_threshold_info(
    config: &Config,
    total_validated: usize,
    issues_found: usize,
    quiet: bool,
) -> String {
    let mut out = String::new();

    if let Some(threshold) = config.failure_threshold {
        // Guard the 0/0 case; NaN would make every comparison below false.
        let failure_rate = if total_validated == 0 {
            0.0
        } else {
            (issues_found as f64 / total_validated as f64) * 100.0
        };

        if !quiet {
            if failure_rate > threshold {
                let _ = writeln!(
                    out,
                    "\n{} Failure rate {:.1}% exceeds threshold {:.1}% ({}/{} URLs failed)",
                    colorize("❌", Colors::BRIGHT_RED),
                    failure_rate,
                    threshold,
                    issues_found,
                    total_validated
                );
            } else if issues_found > 0 {
                let _ = writeln!(
                    out,
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

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::constants::output_formats;
    use crate::core::types::UrlLocation;
    use crate::validation::validator::ValidationResult;
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

        let out = render_config_info(&config, 4, &paths);

        assert!(out.contains("Using threads: 4"));
        assert!(out.contains("Using timeout (seconds): 5"));
        assert!(out.contains("Allow timeout: false"));
        assert!(out.contains("test.md"));
        assert!(out.contains("1 file"), "should be singular for one file");
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

        let out = render_config_info(&config, 8, &paths);

        assert!(out.contains("Using threads: 8"));
        assert!(out.contains("Using timeout (seconds): 60"));
        assert!(out.contains("Allow timeout: true"));
        assert!(out.contains("Retry attempts: 3"));
        assert!(out.contains("Retry delay (ms): 2000"));
        assert!(out.contains("Rate limit delay (ms): 100"));
        assert!(out.contains("Use HEAD requests: true"));
        assert!(out.contains("Skip SSL verification: true"));
        assert!(out.contains("User agent: Custom Agent"));
        assert!(out.contains("Proxy: http://proxy:8080"));
        assert!(out.contains("Allowlist: 1 URLs"));
        assert!(out.contains("[200, 404]"));
        assert!(out.contains("2 files"), "should be plural for two files");
    }

    #[test]
    fn test_display_config_info_many_files() {
        let config = Config::default();
        let paths: Vec<PathBuf> = (1..=15)
            .map(|i| PathBuf::from(format!("file{i}.md")))
            .collect();

        let out = render_config_info(&config, 4, &paths);

        assert!(out.contains("15 files"));
        // Only the first 10 are listed, the rest are summarised.
        assert!(out.contains("file10.md"));
        assert!(!out.contains("file11.md"));
        assert!(out.contains("... and 5 more files"));
    }

    #[test]
    fn test_display_config_info_single_file() {
        let config = Config::default();
        let paths = vec![PathBuf::from("single.md")];

        let out = render_config_info(&config, 2, &paths);

        assert!(out.contains("Using threads: 2"));
        assert!(out.contains("single.md"));
        assert!(out.contains("1 file"));
        assert!(!out.contains("1 files"), "singular, not plural");
    }

    #[test]
    fn test_display_url_discovery_same_count() {
        let url_locations = vec![UrlLocation {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
        }];

        let out = render_url_discovery(1, 1, &url_locations);

        // Equal counts render the short form with no "in total" suffix.
        assert!(out.contains("1 unique URLs"));
        assert!(!out.contains("in total"));
        assert!(out.contains("https://example.com"));
    }

    #[test]
    fn test_display_url_discovery_different_count() {
        let url_locations = vec![UrlLocation {
            url: "https://example.com".to_string(),
            file_name: "test.md".to_string(),
            line: 1,
        }];

        let out = render_url_discovery(1, 3, &url_locations);

        // Differing counts surface both numbers so duplicates are visible.
        assert!(out.contains("1 unique URLs, 3 in total"));
        assert!(out.contains("https://example.com"));
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

        let out = render_url_discovery(2, 5, &url_locations);

        assert!(out.contains("2 unique URLs, 5 in total"));
        assert!(out.contains("https://example.com"));
        assert!(out.contains("https://google.com"));
        // URLs are numbered in order.
        assert!(out.contains("1."));
        assert!(out.contains("2."));
    }

    #[test]
    fn test_display_url_discovery_empty() {
        let url_locations = vec![];
        let out = render_url_discovery(0, 0, &url_locations);

        assert!(out.contains("0 unique URLs"));
        assert!(!out.contains("http"), "no URLs to list");
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
        let out = render_results(&results, output_formats::MINIMAL, false, &config, &metadata);

        assert_eq!(out, "404 https://example.com\n");
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
        let out = render_results(&results, output_formats::JSON, false, &config, &metadata);

        let v: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(v["status"], "failure");
        assert_eq!(v["issues"][0]["url"], "https://example.com");
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
        let out = render_results(&results, output_formats::TEXT, false, &config, &metadata);

        // Text format groups and decorates; minimal/JSON markers must be absent.
        assert!(out.contains("Issues"));
        assert!(out.contains("https://example.com"));
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

        let out = render_minimal_output(&results);

        // Minimal format is "<status> <url>" with no colors or decoration.
        assert_eq!(out, "404 https://example.com\n");
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

        let out = render_minimal_output(&results);

        // With no status code the description stands in for it.
        assert_eq!(out, "Connection timeout https://example.com\n");
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

        let out = render_minimal_output(&results);

        // Neither available: a stable ERROR marker rather than a panic.
        assert_eq!(out, "ERROR https://example.com\n");
    }

    #[test]
    fn test_display_minimal_output_empty() {
        let results = vec![];
        let out = render_minimal_output(&results);

        assert!(out.is_empty(), "no results means no output");
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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
        let json = render_json_output(&results, &metadata).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["validated"], metadata.total_validated);
        assert_eq!(v["urls"]["failed"], metadata.issues_found);
        assert_eq!(
            v["issues"].as_array().expect("issues array").len(),
            results.len()
        );
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

        let out = render_text_output(&results, false, &config, 10, 0);

        assert!(out.contains("No issues found"));
    }

    #[test]
    fn test_display_text_output_empty_quiet() {
        let results = vec![];
        let config = Config::default();

        let out = render_text_output(&results, true, &config, 10, 0);

        assert!(out.is_empty(), "quiet mode prints nothing, got: {out:?}");
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

        let out = render_text_output(&results, false, &config, 10, 1);

        assert!(out.contains("Issues"));
        assert!(!out.contains("No issues found"));
    }

    #[test]
    fn test_render_grouped_issues_buckets_every_status_class() {
        // Pins the 3xx/4xx/5xx/other/network bucketing in one place. Every
        // grouped-issues test previously dropped its output, so a status
        // landing in the wrong bucket would have gone unnoticed.
        let mk = |code: Option<u16>, url: &str| ValidationResult {
            url: url.to_string(),
            file_name: "test.md".to_string(),
            line: 1,
            status_code: code,
            description: code.map(|_| String::new()).or(Some("dns failure".into())),
        };

        let results = vec![
            mk(Some(301), "https://redirect.example"),
            mk(Some(404), "https://client.example"),
            mk(Some(503), "https://server.example"),
            mk(Some(101), "https://other.example"),
            mk(None, "https://network.example"),
        ];

        let out = render_grouped_issues(&results);

        for (heading, url) in [
            ("Redirect Issues", "https://redirect.example"),
            ("Client Errors (4xx)", "https://client.example"),
            ("Server Errors (5xx)", "https://server.example"),
            ("Other HTTP Issues", "https://other.example"),
            ("Network/Connection Errors", "https://network.example"),
        ] {
            assert!(out.contains(heading), "missing heading: {heading}");
            assert!(out.contains(url), "missing url: {url}");
        }
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

        let out = render_grouped_issues(&results);

        assert!(out.contains("Client Errors (4xx)"));
        assert!(out.contains("404"), "status 404 should be listed");
        assert!(out.contains("403"), "status 403 should be listed");
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

        let out = render_grouped_issues(&results);

        assert!(out.contains("Server Errors (5xx)"));
        assert!(out.contains("500"), "status 500 should be listed");
        assert!(out.contains("502"), "status 502 should be listed");
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

        let out = render_grouped_issues(&results);

        assert!(out.contains("Redirect Issues"));
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

        let out = render_grouped_issues(&results);

        assert!(out.contains("Network/Connection Errors"));
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

        let out = render_grouped_issues(&results);

        assert!(out.contains("Other HTTP Issues"));
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

        let out = render_grouped_issues(&results);

        // Mixed input must produce more than one group section.
        let headings = [
            "Network/Connection Errors",
            "Redirect Issues",
            "Client Errors (4xx)",
            "Server Errors (5xx)",
            "Other HTTP Issues",
        ]
        .iter()
        .filter(|h| out.contains(**h))
        .count();
        assert!(headings >= 2, "expected multiple groups, got: {out}");
    }

    #[test]
    fn test_display_grouped_issues_empty() {
        let results = vec![];
        let out = render_grouped_issues(&results);

        assert!(out.is_empty(), "no issues means no groups, got: {out:?}");
    }

    #[test]
    fn test_display_failure_threshold_info_no_threshold() {
        let config = Config::default();
        let out = render_failure_threshold_info(&config, 100, 10, false);

        // With no threshold configured there is nothing to report.
        assert!(out.is_empty(), "expected no output, got: {out:?}");
    }

    #[test]
    fn test_display_failure_threshold_info_exceeds_threshold() {
        let config = Config {
            failure_threshold: Some(5.0),
            ..Default::default()
        };

        let out = render_failure_threshold_info(&config, 100, 10, false); // 10% > 5%

        assert!(out.contains("exceeds threshold"));
        assert!(out.contains("10.0%"));
        assert!(out.contains("5.0%"));
        assert!(out.contains("(10/100 URLs failed)"));
    }

    #[test]
    fn test_display_failure_threshold_info_within_threshold() {
        let config = Config {
            failure_threshold: Some(15.0),
            ..Default::default()
        };

        let out = render_failure_threshold_info(&config, 100, 10, false); // 10% < 15%

        assert!(out.contains("is within threshold"));
        assert!(!out.contains("exceeds"));
        assert!(out.contains("10.0%"));
        assert!(out.contains("15.0%"));
    }

    #[test]
    fn test_display_failure_threshold_info_quiet() {
        let config = Config {
            failure_threshold: Some(5.0),
            ..Default::default()
        };

        let out = render_failure_threshold_info(&config, 100, 10, true);

        // --quiet suppresses the threshold summary entirely.
        assert!(
            out.is_empty(),
            "quiet mode should print nothing, got: {out:?}"
        );
    }

    #[test]
    fn test_display_failure_threshold_info_no_issues() {
        let config = Config {
            failure_threshold: Some(5.0),
            ..Default::default()
        };

        let out = render_failure_threshold_info(&config, 100, 0, false);

        // Zero failures is neither "exceeds" nor worth an "is within" note.
        assert!(out.is_empty(), "expected no output, got: {out:?}");
    }

    #[test]
    fn test_display_failure_threshold_info_edge_cases() {
        let mut config = Config {
            failure_threshold: Some(0.0),
            ..Default::default()
        };

        // 100% > 0% -> exceeded.
        let out = render_failure_threshold_info(&config, 1, 1, false);
        assert!(out.contains("exceeds threshold"));

        // 100% == 100% -> NOT exceeded; the comparison is strictly `>`.
        config.failure_threshold = Some(100.0);
        let out = render_failure_threshold_info(&config, 100, 100, false);
        assert!(
            out.contains("is within threshold"),
            "a rate exactly equal to the threshold must not fail, got: {out:?}"
        );

        // 0 validated would be 0/0 = NaN; must not report a failure.
        config.failure_threshold = Some(0.0);
        let out = render_failure_threshold_info(&config, 0, 0, false);
        assert!(
            !out.contains("exceeds threshold"),
            "zero URLs must not exceed any threshold, got: {out:?}"
        );
    }

    fn meta(validated: usize, issues: usize) -> DisplayMetadata {
        DisplayMetadata {
            total_validated: validated,
            issues_found: issues,
            files_processed: 1,
            total_urls_found: validated,
            unique_urls_found: validated,
        }
    }

    #[test]
    fn test_render_json_escapes_special_characters() {
        // Regression: the hand-rolled writer emitted raw quotes/backslashes,
        // producing JSON that no downstream parser could read.
        let results = vec![ValidationResult {
            url: "https://example.com/a\"b".to_string(),
            line: 7,
            file_name: r#"we"ird\path.md"#.to_string(),
            status_code: None,
            description: Some("error: \"quoted\"\nsecond line\ttabbed".to_string()),
        }];

        let json = render_json_output(&results, &meta(1, 1)).expect("serialization");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("must be valid JSON");

        let issue = &parsed["issues"][0];
        assert_eq!(issue["url"], "https://example.com/a\"b");
        assert_eq!(issue["file"], r#"we"ird\path.md"#);
        assert_eq!(
            issue["description"],
            "error: \"quoted\"\nsecond line\ttabbed"
        );
        assert_eq!(issue["line"], 7);
        assert_eq!(issue["status_code"], serde_json::Value::Null);
    }

    #[test]
    fn test_render_json_schema_is_stable() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 3,
            file_name: "README.md".to_string(),
            status_code: Some(404),
            description: Some("Not Found".to_string()),
        }];

        let json = render_json_output(&results, &meta(4, 1)).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(v["files"]["total"], 1);
        assert_eq!(v["files"]["processed"], 1);
        assert_eq!(v["urls"]["total_found"], 4);
        assert_eq!(v["urls"]["unique"], 4);
        assert_eq!(v["urls"]["validated"], 4);
        assert_eq!(v["urls"]["failed"], 1);
        assert_eq!(v["urls"]["success_rate"], 75.0);
        assert_eq!(v["status"], "failure");
        assert_eq!(v["issues"][0]["status_code"], 404);
        assert_eq!(v["issues"][0]["description"], "Not Found");
    }

    #[test]
    fn test_render_json_success_has_empty_issues() {
        let json = render_json_output(&[], &meta(3, 0)).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(v["status"], "success");
        assert_eq!(v["urls"]["success_rate"], 100.0);
        assert!(
            v["issues"]
                .as_array()
                .expect("issues is an array")
                .is_empty()
        );
    }

    #[test]
    fn test_render_json_description_is_always_a_string() {
        // The pre-serde output used `.unwrap_or("")`, so consumers may call
        // string methods on this field without a null check. Emitting `null`
        // for `None` would break them.
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: Some(404),
            description: None,
        }];

        let json = render_json_output(&results, &meta(1, 1)).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(v["issues"][0]["description"], "");
        assert!(
            v["issues"][0]["description"].is_string(),
            "description must never be null"
        );
        // status_code, by contrast, was already null in the old format.
        assert_eq!(v["issues"][0]["status_code"], 404);
    }

    #[test]
    fn test_render_json_status_code_is_null_when_absent() {
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: None,
            description: Some("dns failure".to_string()),
        }];

        let json = render_json_output(&results, &meta(1, 1)).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");

        assert!(v["issues"][0]["status_code"].is_null());
        assert_eq!(v["issues"][0]["description"], "dns failure");
    }

    #[test]
    fn test_render_json_is_a_single_line() {
        // Line-oriented consumers (grep/head) depend on this.
        let results = vec![ValidationResult {
            url: "https://example.com".to_string(),
            line: 1,
            file_name: "test.md".to_string(),
            status_code: Some(500),
            description: Some("boom".to_string()),
        }];

        let json = render_json_output(&results, &meta(2, 1)).expect("serialization");
        assert!(!json.contains('\n'), "JSON output must not be multi-line");
    }

    #[test]
    fn test_render_json_no_urls_reports_full_success_rate() {
        let json = render_json_output(&[], &meta(0, 0)).expect("serialization");
        let v: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(v["urls"]["success_rate"], 100.0);
    }
}
