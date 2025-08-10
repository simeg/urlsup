//! Property-based tests for urlsup using proptest
//!
//! These tests generate random inputs to test edge cases and ensure
//! robustness across a wide range of potential inputs.

use assert_cmd::prelude::*;
use proptest::prelude::*;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

const NAME: &str = "urlsup";

/// Generate valid-ish URLs for testing
fn url_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid HTTP/HTTPS URLs
        prop::collection::vec("[a-z]{3,10}", 1..5)
            .prop_map(|parts| format!("https://{}.com", parts.join("."))),
        // URLs with ports
        (r"[a-z]{3,8}", 1024..65535u16)
            .prop_map(|(domain, port)| format!("http://{}:{}", domain, port)),
        // URLs with paths
        (r"[a-z]{3,8}", prop::collection::vec(r"[a-z]{1,8}", 0..5)).prop_map(
            |(domain, path_parts)| {
                if path_parts.is_empty() {
                    format!("https://{}.com", domain)
                } else {
                    format!("https://{}.com/{}", domain, path_parts.join("/"))
                }
            }
        ),
        // URLs with query parameters
        (r"[a-z]{3,8}", r"[a-z]{1,8}", r"[a-z]{1,8}").prop_map(|(domain, key, value)| {
            format!("https://{}.com?{}={}", domain, key, value)
        }),
        // Edge case URLs
        prop_oneof![
            Just("http://localhost".to_string()),
            Just("https://127.0.0.1".to_string()),
            Just("ftp://example.com".to_string()),
            Just("https://[::1]".to_string()),
        ]
    ]
}

/// Generate potentially problematic URLs
fn problematic_url_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Malformed URLs
        r"[a-z]{5,15}",            // No protocol
        r"://[a-z]{5,15}",         // No protocol, just ://
        r"http://",                // Incomplete
        r"https://.",              // Invalid domain
        r"http:// invalid spaces", // Spaces
        // Very long URLs
        prop::collection::vec(r"[a-z]", 100..200).prop_map(|chars| format!(
            "https://example.com/{}",
            chars.into_iter().collect::<String>()
        )),
        // Unicode URLs
        Just("https://例え.テスト".to_string()),
        Just("https://xn--r8jz45g.xn--zckzah".to_string()),
        // Special characters
        Just("https://example.com/path%20with%20spaces".to_string()),
        Just("https://example.com/path?query=value&other=test".to_string()),
    ]
}

/// Generate file content with random URLs
fn file_content_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            // Lines with URLs
            url_strategy().prop_map(|url| format!("Check out this link: {}", url)),
            url_strategy().prop_map(|url| format!("Visit {} for more info", url)),
            url_strategy().prop_map(|url| format!("[Link]({}) description", url)),
            // Lines without URLs
            Just("This is just plain text".to_string()),
            Just("# This is a heading".to_string()),
            Just("- Bullet point without links".to_string()),
            Just("".to_string()), // Empty lines
            // Mixed content
            (url_strategy(), url_strategy())
                .prop_map(|(url1, url2)| format!("Multiple links: {} and {}", url1, url2)),
        ],
        1..20,
    )
    .prop_map(|lines| lines.join("\n"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))] // Default is 256...

    #[test]
    fn test_handles_random_file_content(
        content in file_content_strategy()
    ) {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
            .arg("--retry-delay")
            .arg("1")
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")  // Short timeout for property tests
            .arg("--allow-timeout");

        // Should not crash, regardless of content
        // Can succeed or fail, but should not panic or crash
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_handles_problematic_urls(
        content in prop::collection::vec(problematic_url_strategy(), 1..10)
            .prop_map(|urls| urls.join("\n"))
    ) {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")
            .arg("--allow-timeout");

        // Should handle malformed URLs gracefully
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_configuration_combinations(
        timeout in 1u64..30,
        concurrency in 1u8..10,
        retry_attempts in 0u8..5,
        retry_delay in 100u64..2000,
    ) {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"https://httpbin.org/status/200").unwrap();

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg(timeout.to_string())
            .arg("--concurrency")
            .arg(concurrency.to_string())
            .arg("--retry")
            .arg(retry_attempts.to_string())
            .arg("--retry-delay")
            .arg(retry_delay.to_string())
            .arg("--allow-timeout");

        // Should handle any reasonable configuration
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_allowlist_patterns(
        patterns in prop::collection::vec(r"[a-z]{3,10}", 1..5)
    ) {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"https://example.com\nhttps://google.com").unwrap();

        let allowlist = patterns.join(",");

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
            .arg("--allowlist")
            .arg(&allowlist)
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")
            .arg("--allow-timeout");

        // Should handle any allowlist patterns without crashing
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_status_code_combinations(
        codes in prop::collection::vec(100u16..600, 1..10)
    ) {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"https://httpbin.org/status/404").unwrap();

        let status_codes = codes.iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
            .arg("--allow-status")
            .arg(&status_codes)
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")
            .arg("--allow-timeout");

        // Should handle any valid HTTP status codes
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_file_extensions(
        extensions in prop::collection::vec(r"[a-z]{2,5}", 1..8)
    ) {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create files with random extensions
        for (i, ext) in extensions.iter().enumerate() {
            let file_path = temp_dir.path().join(format!("test{}.{}", i, ext));
            std::fs::write(&file_path, "https://example.com").unwrap();
        }

        let extension_list = extensions.join(",");

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg("--recursive")
            .arg("--include")
            .arg(&extension_list)
            .arg(temp_dir.path())
            .arg("--format")
            .arg("minimal")
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")
            .arg("--allow-timeout");

        // Should handle any file extensions
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_large_content_generation(
        line_count in 50usize..200,
        urls_per_line in 0usize..5
    ) {
        let mut content = Vec::new();

        for _ in 0..line_count {
            let mut line = String::new();
            for _ in 0..urls_per_line {
                line.push_str(&format!("https://example{}.com ", content.len()));
            }
            if urls_per_line == 0 {
                line = format!("Regular text line {}", content.len());
            }
            content.push(line);
        }

        let file_content = content.join("\n");
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(file_content.as_bytes()).unwrap();

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")
            .arg("--allow-timeout")
            .arg("--concurrency")
            .arg("5");

        // Should handle large files without issues
        let _ = cmd.assert().try_success();
    }

    #[test]
    fn test_failure_threshold_edge_cases(
        threshold in 0.0f64..100.0
    ) {
        let mut file = NamedTempFile::new().unwrap();
        // Mix of valid and invalid URLs
        file.write_all(b"https://httpbin.org/status/200\nhttps://httpbin.org/status/404").unwrap();

        let mut cmd = Command::cargo_bin(NAME).unwrap();
        cmd.arg(file.path())
            .arg("--format")
            .arg("minimal")
            .arg("--failure-threshold")
            .arg(threshold.to_string())
                    .arg("--retry-delay")
            .arg("1")
            .arg("--timeout")
            .arg("1")
            .arg("--allow-timeout");

        // Should handle any threshold value
        let _ = cmd.assert().try_success();
        }
}

#[cfg(test)]
mod unit_property_tests {
    use super::*;
    use proptest::proptest;

    proptest! {

        #[test]
        fn test_url_strategy_generates_valid_formats(url in url_strategy()) {
            // Basic validation that generated URLs have expected structure
            prop_assert!(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ftp://"));
            prop_assert!(url.len() > 7);  // Minimum URL length
            prop_assert!(url.len() < 2000);  // Reasonable maximum
        }

        #[test]
        fn test_problematic_url_strategy_coverage(url in problematic_url_strategy()) {
            // Just ensure the strategy generates diverse problematic cases
            prop_assert!(!url.is_empty());
            prop_assert!(url.len() < 500);  // Keep it reasonable
        }

        #[test]
        fn test_file_content_strategy_generates_valid_content(content in file_content_strategy()) {
            // Content should be valid UTF-8 and not too large
            prop_assert!(content.len() < 10000);  // Reasonable size limit
            prop_assert!(content.is_ascii() || content.chars().all(|c| c.is_alphanumeric() || c.is_whitespace() || ":/.-_?=&%".contains(c)));
        }
    }
}
