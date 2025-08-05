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
        cmd.assert().failure().stderr(contains(
            "error: the following required arguments were not provided:\n  <FILES>...",
        ));
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

        cmd.arg(file.path());

        cmd.assert()
            .success()
            .stdout(contains("✓ No issues found!"));
        Ok(())
    }

    #[tokio::test]
    async fn test_output__when_single_issue() -> TestResult {
        let mut server = Server::new_async().await;
        let _m404 = server.mock("GET", "/404").with_status(404).create();
        let endpoint = server.url() + "/404";
        let mut file = tempfile::NamedTempFile::new()?;
        let file_name = file.path().display().to_string();
        file.write_all(endpoint.as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path());

        cmd.assert().failure();
        cmd.assert().failure().stdout(contains("✗ Found 1 issues:"));
        cmd.assert().failure().stdout(contains(format!(
            "404 - {}/404 - {} - L1",
            server.url(),
            file_name
        )));
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

        cmd.arg(file.path());

        cmd.assert().failure();
        cmd.assert().failure().stdout(contains("✗ Found 2 issues:"));
        // Order is not deterministic so can't assert it
        cmd.assert()
            .failure()
            .stdout(contains(format!("404 - {}/404", server.url())));
        cmd.assert()
            .failure()
            .stdout(contains(format!("401 - {}/401", server.url())));
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

        cmd.arg(file.path()).arg("--allowlist").arg(format!(
            "{}/401,{}/404",
            server.url(),
            server.url()
        ));

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("✓ No issues found!"));
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

        cmd.arg(file.path()).arg("--allow-status").arg("401,404");

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("✓ No issues found!"));
        Ok(())
    }

    #[test]
    fn test_output__when_non_existing_file_provided() {
        let mut cmd = Command::cargo_bin(NAME).unwrap();

        cmd.arg("some-file-that-doesnt-exist");

        cmd.assert().failure();
        cmd.assert().failure().stderr("error: invalid value \'some-file-that-doesnt-exist\' for \'<FILES>...\': File not found [\"some-file-that-doesnt-exist\"]\n\nFor more information, try \'--help\'.\n");
    }

    #[test]
    fn test_output__when_too_big_timeout_provided() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut cmd = Command::cargo_bin(NAME).unwrap();
        let too_big_timeout = 118446744073709551616_u128.to_string();

        cmd.arg(file.path()).arg("--timeout").arg(too_big_timeout);

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains(
            "Error: Could not parse timeout '118446744073709551616' as a valid number",
        ));
    }

    #[test]
    fn test_output__when_non_number_allowed_status_code() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut cmd = Command::cargo_bin(NAME).unwrap();
        let non_number = "not-a-number";

        cmd.arg(file.path()).arg("--allow-status").arg(non_number);

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains(
            "Error: Could not parse status code 'not-a-number' as a valid number",
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
            .arg("--allow-timeout");

        cmd.assert()
            .success()
            .stdout(contains("✓ No issues found!"));
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

        cmd.arg("--recursive").arg(temp_dir.path());

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("✓ No issues found!"));
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
            .arg(temp_dir.path());

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("✓ No issues found!"));
        Ok(())
    }
}
