mod cli {
    #![allow(non_snake_case)]

    use assert_cmd::prelude::*;
    use mockito::Server;
    use predicates::str::{contains, ends_with, starts_with};

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
            .stdout(contains("Found 1 unique URL(s), 1 in total"));
        cmd.assert().success().stdout(ends_with("No issues!\n"));
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
        cmd.assert()
            .failure()
            .stdout(contains("Found 1 unique URL(s), 1 in total"));
        cmd.assert().failure().stdout(ends_with(format!(
            "> Issues\n   1. 404 - {}/404 - {} - L1\n",
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
        file.write_all(format!("{} {}", endpoint_404, endpoint_401).as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path());

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stdout(contains("Found 2 unique URL(s), 2 in total"));
        cmd.assert().failure().stdout(contains("> Issues"));
        // Order is not deterministic so can't assert it
        cmd.assert()
            .failure()
            .stdout(contains(&format!("404 - {}/404", server.url())));
        cmd.assert()
            .failure()
            .stdout(contains(&format!("401 - {}/401", server.url())));
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
        file.write_all(format!("{} {} {}", endpoint_200, endpoint_401, endpoint_404).as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--white-list").arg(&format!(
            "{}/401,{}/404",
            server.url(),
            server.url()
        ));

        cmd.assert().success();
        cmd.assert().success().stdout(contains(&format!(
            "Ignoring white listed URL(s)\n   1. {}/401\n   2. {}/404",
            server.url(),
            server.url()
        )));
        cmd.assert().success().stdout(ends_with("No issues!\n"));
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
        file.write_all(format!("{} {} {}", endpoint_200, endpoint_401, endpoint_404).as_bytes())?;
        let mut cmd = Command::cargo_bin(NAME)?;

        cmd.arg(file.path()).arg("--allow").arg("401,404");

        cmd.assert().success();
        cmd.assert()
            .success()
            .stdout(contains("Allowing HTTP status codes\n   1. 401\n   2. 404"));
        cmd.assert().success().stdout(ends_with("No issues!\n"));
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
        let too_big_timeout = (118446744073709551616 as u128).to_string();

        cmd.arg(file.path()).arg("--timeout").arg(too_big_timeout);

        cmd.assert().failure();
        cmd.assert().failure().stderr(contains(
            "Could not parse 118446744073709551616 into an int (u64)",
        ));
    }

    #[test]
    fn test_output__when_non_number_allowed_status_code() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut cmd = Command::cargo_bin(NAME).unwrap();
        let non_number = "not-a-number";

        cmd.arg(file.path()).arg("--allow").arg(non_number);

        cmd.assert().failure();
        cmd.assert()
            .failure()
            .stderr(contains("Could not parse status code to int (u16)"));
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
            .arg("--threads")
            .arg("10")
            .arg("--timeout")
            .arg("20")
            .arg("--allow")
            .arg("200,404")
            .arg("--white-list")
            .arg("http://some-url.com")
            .arg("--allow-timeout");

        cmd.assert()
            .success()
            .stdout(starts_with("> Using threads: 10\n> Using timeout (seconds): 20\n> Allow timeout: true\n> Ignoring white listed URL(s)\n   1. http://some-url.com\n> Allowing HTTP status codes\n   1. 200\n   2. 404"));
        Ok(())
    }
}
