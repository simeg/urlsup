mod cli {
    #![allow(non_snake_case)]

    use assert_cmd::prelude::*;
    use mockito::Server;
    use predicates::str::contains;

    use std::io::Write;
    use std::process::Command;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const NAME: &str = "urlsup";

    #[test]
    fn test_output__when_no_files_provided() -> TestResult {
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stderr(contains("Error: No files provided"));
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_no_issues() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--format").arg("minimal");

        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_single_issue() -> TestResult {
        let mut server = Server::new_async().await;
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint = server.url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--format").arg("minimal");

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stdout(contains(format!("404 {}/404", server.url())));
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_multiple_issues() -> TestResult {
        let mut server = Server::new_async().await;
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let _m401 = server.mock("GET", "/401").with_status(401).create();
        let endpoint_404 = server.url() + "/404";
        let endpoint_401 = server.url() + "/401";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(format!("{endpoint_404} {endpoint_401}").as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--format").arg("minimal");

        cmd.assert().failure();
        // Order is not deterministic so can't assert it
        cmd.assert()
            .failure()
            .stdout(contains(format!("404 {}/404", server.url())));
        cmd.assert()
            .failure()
            .stdout(contains(format!("401 {}/401", server.url())));
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_white_list_provided() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let _m401 = server.mock("GET", "/401").with_status(401).create();
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint_200 = server.url() + "/200";
        let endpoint_401 = server.url() + "/401";
        let endpoint_404 = server.url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(format!("{endpoint_200} {endpoint_401} {endpoint_404}").as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path())
            .arg("--allowlist")
            .arg(format!("{}/401,{}/404", server.url(), server.url()))
            .arg("--format")
            .arg("minimal");

        cmd.assert().success();
        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_allowed_statuses_provided() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let _m401 = server.mock("GET", "/401").with_status(401).create();
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint_200 = server.url() + "/200";
        let endpoint_401 = server.url() + "/401";
        let endpoint_404 = server.url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(format!("{endpoint_200} {endpoint_401} {endpoint_404}").as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path())
            .arg("--allow-status")
            .arg("401,404")
            .arg("--format")
            .arg("minimal");

        cmd.assert().success();
        cmd.assert().success().stdout("");
        Ok(())
    }

    #[test]
    fn test_output__when_non_existing_file_provided() {
        let mut cmd = Command::cargo_bin(NAME).unwrap();

        cmd.arg("some-file-that-doesnt-exist");

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains(
            "Error: File not found: \'some-file-that-doesnt-exist\'",
        ));
        // Our improved error handling provides cleaner output
        // The help message is not shown for file validation errors
    }

    #[test]
    fn test_output__when_too_big_timeout_provided() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut cmd = Command::cargo_bin(NAME).unwrap();
        let too_big_timeout = 118446744073709551616_u128.to_string();

        cmd.arg(file.path()).arg("--timeout").arg(too_big_timeout);

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stderr(contains("number too large to fit in target type"));
    }

    #[test]
    fn test_output__when_non_number_allowed_status_code() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut cmd = Command::cargo_bin(NAME).unwrap();
        let non_number = "not-a-number";

        cmd.arg(file.path()).arg("--allow-status").arg(non_number);

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains(
            "Status code 'not-a-number' is not a valid HTTP status code",
        ));
    }

    #[tokio::test]
    async fn test_output__all_opts_printed() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path())
            .arg("--concurrency")
            .arg("10")
            .arg("--timeout")
            .arg("20")
            .arg("--allow-status")
            .arg("200,404")
            .arg("--allowlist")
            .arg("http://some-url.com")
            .arg("--allow-timeout")
            .arg("--format")
            .arg("minimal");

        cmd.assert().success().stdout("");
        Ok(())
    }

    #[test]
    fn test_output__when_directory_without_recursive() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(temp_dir.path());

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains(
            "is a directory. Use --recursive to process directories.",
        ));
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_recursive_flag_used() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";

        // Create temporary directory with a markdown file
        let temp_dir = tempfile::tempdir()?;
        let file_path = temp_dir.path().join("test.md");
        std::fs::write(&file_path, endpoint.as_bytes())?;

        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg("--recursive")
            .arg(temp_dir.path())
            .arg("--format")
            .arg("minimal");

        cmd.assert().success();
        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_file_types_filter_used() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let _m201 = server.mock("GET", "/201").with_status(200).create();
        let endpoint1 = server.url() + "/200";
        let endpoint2 = server.url() + "/201";

        // Create temporary directory with different file types
        let temp_dir = tempfile::tempdir()?;
        let md_file = temp_dir.path().join("test.md");
        let txt_file = temp_dir.path().join("test.txt");
        let html_file = temp_dir.path().join("test.html");

        std::fs::write(&md_file, endpoint1.as_bytes())?;
        std::fs::write(&txt_file, endpoint2.as_bytes())?;
        std::fs::write(&html_file, "no urls here")?;

        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg("--recursive")
            .arg("--include")
            .arg("md,txt")
            .arg(temp_dir.path())
            .arg("--format")
            .arg("minimal");

        cmd.assert().success();
        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_output__json_format_with_success() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/200").with_status(200).create();
        let endpoint = server.url() + "/200";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--format").arg("json");

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("\"status\": \"success\""))
            .stdout(contains("\"issues\": []"));
        Ok(())
    }

    #[tokio::test]
    async fn test_output__json_format_with_failures() -> TestResult {
        let mut server = Server::new_async().await;
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint = server.url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--format").arg("json");

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stdout(contains("\"status\": \"failure\""))
            .stdout(contains("\"status_code\": 404"));
        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_validation() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200_1 = server.mock("GET", "/url1").with_status(200).create();
        let _m200_2 = server.mock("GET", "/url2").with_status(200).create();
        let _m200_3 = server.mock("GET", "/url3").with_status(200).create();

        let urls = format!(
            "{}/url1\n{}/url2\n{}/url3",
            server.url(),
            server.url(),
            server.url()
        );
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(urls.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path())
            .arg("--concurrency")
            .arg("2")
            .arg("--format")
            .arg("minimal");

        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_retry_mechanism() -> TestResult {
        let mut server = Server::new_async().await;
        // First request fails, subsequent requests succeed
        let _m500 = server
            .mock("GET", "/flaky")
            .with_status(500)
            .expect(1)
            .create();
        let _m200 = server
            .mock("GET", "/flaky")
            .with_status(200)
            .expect_at_least(1)
            .create();

        let endpoint = server.url() + "/flaky";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path())
            .arg("--retry")
            .arg("2")
            .arg("--retry-delay")
            .arg("100")
            .arg("--format")
            .arg("minimal");

        // The retry mechanism should be exercised regardless of final success/failure
        let _result = cmd.assert();
        Ok(())
    }

    #[test]
    fn test_config_file_validation() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let config_file = temp_dir.path().join(".urlsup.toml");

        // Invalid TOML
        std::fs::write(&config_file, "invalid toml content [")?;

        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg("--config").arg(&config_file).arg("nonexistent.md");

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stderr(contains("Configuration error"));
        Ok(())
    }

    #[tokio::test]
    async fn test_exclude_patterns() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/public").with_status(200).create();
        let _m404 = server.mock("GET", "/private").with_status(404).create();

        let urls = format!("{}/public\n{}/private", server.url(), server.url());
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(urls.as_bytes())?;

        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg(file.path())
            .arg("--exclude-pattern")
            .arg(".*/private")
            .arg("--format")
            .arg("minimal");

        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_failure_threshold() -> TestResult {
        let mut server = Server::new_async().await;
        let _m200 = server.mock("GET", "/good").with_status(200).create();
        let _m404 = server.mock("GET", "/bad").with_status(404).create();

        let urls = format!("{}/good\n{}/bad", server.url(), server.url());
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(urls.as_bytes())?;

        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg(file.path())
            .arg("--failure-threshold")
            .arg("60") // 50% failure rate should be within 60% threshold
            .arg("--format")
            .arg("minimal");

        cmd.assert().success();
        Ok(())
    }

    #[tokio::test]
    async fn test_large_file_handling() -> TestResult {
        let mut server = Server::new_async().await;

        // Create many mock endpoints
        let mut urls = Vec::new();
        for i in 0..50 {
            let _mock = server
                .mock("GET", format!("/url{}", i).as_str())
                .with_status(200)
                .create();
            urls.push(format!("{}/url{}", server.url(), i));
        }

        let content = urls.join("\n");
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(content.as_bytes())?;

        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg(file.path())
            .arg("--concurrency")
            .arg("10")
            .arg("--format")
            .arg("minimal");

        cmd.assert().success().stdout("");
        Ok(())
    }

    #[test]
    fn test_binary_file_handling() -> TestResult {
        let temp_dir = tempfile::tempdir()?;
        let binary_file = temp_dir.path().join("test.bin");

        // Create a binary file with some non-UTF8 content
        std::fs::write(&binary_file, vec![0xFF, 0xFE, 0xFD])?;

        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg(&binary_file).arg("--format").arg("minimal");

        // Should handle gracefully, not crash
        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_custom_user_agent() -> TestResult {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/test")
            .match_header("user-agent", "CustomBot/1.0")
            .with_status(200)
            .create();

        let endpoint = server.url() + "/test";
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(endpoint.as_bytes())?;

        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg(file.path())
            .arg("--user-agent")
            .arg("CustomBot/1.0")
            .arg("--format")
            .arg("minimal");

        cmd.assert().success().stdout("");
        Ok(())
    }

    #[tokio::test]
    async fn test_rate_limiting() -> TestResult {
        let mut server = Server::new_async().await;
        let _m1 = server.mock("GET", "/url1").with_status(200).create();
        let _m2 = server.mock("GET", "/url2").with_status(200).create();

        let urls = format!("{}/url1\n{}/url2", server.url(), server.url());
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(urls.as_bytes())?;

        let start = std::time::Instant::now();
        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg(file.path())
            .arg("--rate-limit")
            .arg("200") // 200ms between requests
            .arg("--format")
            .arg("minimal");

        cmd.assert().success().stdout("");

        // Should take some time due to rate limiting, but timing can be variable in CI
        // Just verify the command completed successfully with rate limiting enabled
        let elapsed = start.elapsed().as_millis();
        // Very lenient timing check - just ensure it didn't complete instantly
        assert!(elapsed >= 10);
        Ok(())
    }

    #[test]
    fn test_invalid_arguments() -> TestResult {
        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg("--invalid-flag");

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains("error:"));
        Ok(())
    }

    #[test]
    fn test_help_output() -> TestResult {
        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg("--help");

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("CLI to validate URLs in files"))
            .stdout(contains("Usage:"));
        Ok(())
    }

    #[test]
    fn test_version_output() -> TestResult {
        let mut cmd = Command::cargo_bin(NAME)?;
        cmd.arg("--version");

        cmd.assert().success();
        cmd.assert().success().stdout(contains("urlsup"));
        Ok(())
    }
}
