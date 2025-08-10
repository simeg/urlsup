//! Interactive configuration wizard for urlsup
//!
//! Provides a step-by-step guided setup for new users to create
//! optimal configurations for their specific use cases.

use crate::config::Config;
use crate::ui::color::{Colors, colorize};
use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};
use std::fmt;
use std::path::PathBuf;

/// Errors that can occur during wizard execution
#[derive(Debug)]
pub enum WizardError {
    /// IO error during file operations
    Io(std::io::Error),
    /// Dialoguer interaction error
    Dialog(dialoguer::Error),
    /// Configuration serialization error
    Serialization(toml::ser::Error),
}

impl fmt::Display for WizardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Dialog(e) => write!(f, "Dialog error: {}", e),
            Self::Serialization(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for WizardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Dialog(e) => Some(e),
            Self::Serialization(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for WizardError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<dialoguer::Error> for WizardError {
    fn from(error: dialoguer::Error) -> Self {
        Self::Dialog(error)
    }
}

impl From<toml::ser::Error> for WizardError {
    fn from(error: toml::ser::Error) -> Self {
        Self::Serialization(error)
    }
}

/// Result type for wizard operations
type WizardResult<T> = Result<T, WizardError>;

/// Project templates with pre-configured settings
#[derive(Debug, Clone)]
pub struct ProjectTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub config: Config,
    pub file_types: Vec<&'static str>,
}

impl fmt::Display for ProjectTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.name, self.description)
    }
}

/// Available project templates
pub fn get_project_templates() -> Vec<ProjectTemplate> {
    vec![
        ProjectTemplate {
            name: "Documentation Site",
            description: "Static site generators (Jekyll, Hugo, Gatsby, etc.)",
            config: Config {
                timeout: Some(30),
                allow_timeout: Some(false),
                retry_attempts: Some(2),
                retry_delay: Some(1000),
                rate_limit_delay: Some(100),
                failure_threshold: Some(5.0),
                allowed_status_codes: Some(vec![403]), // Common for docs sites
                ..Config::default()
            },
            file_types: vec!["md", "html", "txt", "rst"],
        },
        ProjectTemplate {
            name: "GitHub Repository",
            description: "README, contributing guides, and documentation",
            config: Config {
                timeout: Some(20),
                allow_timeout: Some(true), // External links might be slow
                retry_attempts: Some(3),
                retry_delay: Some(1500),
                rate_limit_delay: Some(200),
                failure_threshold: Some(10.0),
                allowed_status_codes: Some(vec![403, 429]),
                ..Config::default()
            },
            file_types: vec!["md", "txt"],
        },
        ProjectTemplate {
            name: "Blog/Content Site",
            description: "WordPress, medium articles, or content management",
            config: Config {
                timeout: Some(45),
                allow_timeout: Some(true),
                retry_attempts: Some(2),
                retry_delay: Some(2000),
                rate_limit_delay: Some(300), // Be nice to external sites
                failure_threshold: Some(15.0), // More lenient for external content
                allowed_status_codes: Some(vec![403, 429, 503]),
                ..Config::default()
            },
            file_types: vec!["md", "html", "txt"],
        },
        ProjectTemplate {
            name: "API Documentation",
            description: "OpenAPI specs, API guides, endpoint documentation",
            config: Config {
                timeout: Some(60),          // APIs might be slower
                allow_timeout: Some(false), // APIs should be reliable
                retry_attempts: Some(5),    // Retry more for APIs
                retry_delay: Some(1000),
                rate_limit_delay: Some(500),  // Respect API rate limits
                failure_threshold: Some(2.0), // Strict for API docs
                allowed_status_codes: Some(vec![401, 403]), // Auth might be required
                ..Config::default()
            },
            file_types: vec!["md", "yml", "yaml", "json", "html"],
        },
        ProjectTemplate {
            name: "Wiki/Knowledge Base",
            description: "Internal wikis, knowledge bases, documentation hubs",
            config: Config {
                timeout: Some(30),
                allow_timeout: Some(true),
                retry_attempts: Some(3),
                retry_delay: Some(1000),
                rate_limit_delay: Some(150),
                failure_threshold: Some(8.0),
                allowed_status_codes: Some(vec![403, 404, 429]), // Internal links might be restricted
                ..Config::default()
            },
            file_types: vec!["md", "wiki", "txt", "html"],
        },
        ProjectTemplate {
            name: "CI/CD Pipeline",
            description: "Automated validation in continuous integration",
            config: Config {
                timeout: Some(15), // Fast for CI
                allow_timeout: Some(true),
                retry_attempts: Some(1), // Quick failure in CI
                retry_delay: Some(500),
                rate_limit_delay: Some(50),
                failure_threshold: Some(0.0), // Strict by default
                output_format: Some("minimal".to_string()), // CI-friendly output
                ..Config::default()
            },
            file_types: vec!["md", "txt", "html"],
        },
        ProjectTemplate {
            name: "Custom Setup",
            description: "Configure everything manually",
            config: Config::default(),
            file_types: vec!["md"],
        },
    ]
}

/// Configuration wizard builder for step-by-step setup
pub struct ConfigurationWizard {
    theme: ColorfulTheme,
}

impl Default for ConfigurationWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationWizard {
    /// Create a new configuration wizard
    pub fn new() -> Self {
        Self {
            theme: ColorfulTheme::default(),
        }
    }

    /// Run the interactive configuration wizard
    pub fn run(&self) -> WizardResult<()> {
        self.display_welcome();

        let template = self.select_project_template()?;
        let mut config = template.config.clone();
        let mut file_types = template.file_types.clone();

        if template.name != "Custom Setup" {
            file_types = self.select_file_types(&file_types)?;
        }

        let should_customize = self.should_customize_settings(template.name == "Custom Setup")?;
        if should_customize {
            config = self.configure_advanced_settings(config)?;
        }

        let should_setup_filters = self.should_setup_filters()?;
        if should_setup_filters {
            config = self.configure_url_filters(config)?;
        }

        self.generate_and_save_config(&config, &file_types)?;
        self.show_completion_message(&file_types);

        Ok(())
    }

    /// Display welcome message
    fn display_welcome(&self) {
        println!(
            "\n{}",
            colorize("🧙‍♂️ urlsup Configuration Wizard", Colors::BRIGHT_CYAN)
        );
        println!(
            "{}\n",
            colorize("Let's set up urlsup for your project!", Colors::CYAN)
        );
    }

    /// Select project template
    fn select_project_template(&self) -> WizardResult<ProjectTemplate> {
        let templates = get_project_templates();
        let template_names: Vec<&str> = templates.iter().map(|t| t.name).collect();

        println!(
            "{}",
            colorize(
                "📋 What type of project are you setting up?",
                Colors::BRIGHT_WHITE
            )
        );
        let selection = Select::with_theme(&self.theme)
            .items(&template_names)
            .default(0)
            .interact()?;

        let selected_template = templates[selection].clone();
        println!(
            "\n{} {}",
            colorize("✓", Colors::BRIGHT_GREEN),
            colorize(
                &format!("Selected: {}", selected_template.name),
                Colors::GREEN
            )
        );
        println!("{}\n", colorize(selected_template.description, Colors::DIM));

        Ok(selected_template)
    }

    /// Select file types to process
    fn select_file_types(&self, current_types: &[&str]) -> WizardResult<Vec<&'static str>> {
        println!(
            "{}",
            colorize("📁 Which file types should we check?", Colors::BRIGHT_WHITE)
        );
        println!(
            "{}",
            colorize(
                "(Select the file types that contain URLs in your project)",
                Colors::DIM
            )
        );

        const AVAILABLE_TYPES: &[&str] = &[
            "md", "markdown", "txt", "html", "htm", "rst", "adoc", "asciidoc", "wiki", "yml",
            "yaml", "json", "xml", "tex", "org", "textile",
        ];

        let defaults: Vec<bool> = AVAILABLE_TYPES
            .iter()
            .map(|&ext| current_types.contains(&ext))
            .collect();

        let selected_indices = MultiSelect::with_theme(&self.theme)
            .items(AVAILABLE_TYPES)
            .defaults(&defaults)
            .interact()?;

        Ok(selected_indices
            .iter()
            .map(|&i| AVAILABLE_TYPES[i])
            .collect())
    }

    /// Ask if user wants to customize settings
    fn should_customize_settings(&self, is_custom: bool) -> WizardResult<bool> {
        if is_custom {
            Ok(true)
        } else {
            println!(
                "\n{}",
                colorize(
                    "🔧 Would you like to customize the advanced settings?",
                    Colors::BRIGHT_WHITE
                )
            );
            Ok(Confirm::with_theme(&self.theme)
                .with_prompt("Customize advanced settings")
                .default(false)
                .interact()?)
        }
    }

    /// Ask if user wants to setup filters
    fn should_setup_filters(&self) -> WizardResult<bool> {
        Ok(Confirm::with_theme(&self.theme)
            .with_prompt(format!(
                "{} Set up URL allowlists or exclusion patterns?",
                colorize("🎯", Colors::BRIGHT_YELLOW)
            ))
            .default(false)
            .interact()?)
    }

    /// Generate and save configuration file
    fn generate_and_save_config(&self, config: &Config, file_types: &[&str]) -> WizardResult<()> {
        println!(
            "\n{}",
            colorize("💾 Generating configuration...", Colors::BRIGHT_CYAN)
        );

        let config_content = ConfigFileGenerator::new(config, file_types).generate()?;
        let config_path = PathBuf::from(".urlsup.toml");

        if config_path.exists() {
            let overwrite = Confirm::with_theme(&self.theme)
                .with_prompt(format!(
                    "{} .urlsup.toml already exists. Overwrite?",
                    colorize("⚠️", Colors::BRIGHT_YELLOW)
                ))
                .default(false)
                .interact()?;

            if !overwrite {
                println!("{}", colorize("Configuration not saved.", Colors::YELLOW));
                return Ok(());
            }
        }

        std::fs::write(&config_path, config_content)?;

        println!(
            "\n{} {}",
            colorize("✅", Colors::BRIGHT_GREEN),
            colorize("Configuration saved to .urlsup.toml", Colors::BRIGHT_GREEN)
        );

        Ok(())
    }

    /// Show completion message and usage examples
    fn show_completion_message(&self, file_types: &[&str]) {
        UsageExamples::new(file_types).display();
        println!(
            "\n{}",
            colorize(
                "🎉 Setup complete! Happy URL validation!",
                Colors::BRIGHT_GREEN
            )
        );
    }

    /// Configure advanced settings interactively
    fn configure_advanced_settings(&self, mut config: Config) -> WizardResult<Config> {
        println!(
            "\n{}",
            colorize("⚙️ Advanced Configuration", Colors::BRIGHT_WHITE)
        );

        // Timeout
        let timeout: u64 = Input::with_theme(&self.theme)
            .with_prompt("Connection timeout (seconds)")
            .default(config.timeout.unwrap_or(30))
            .interact()?;
        config.timeout = Some(timeout);

        // Concurrency (stored but not used in Config struct)
        let default_threads = num_cpus::get().min(16);
        let _threads: usize = Input::with_theme(&self.theme)
            .with_prompt("Number of concurrent requests")
            .default(default_threads)
            .validate_with(Self::validate_thread_count)
            .interact()?;
        // Note: threads is not directly in Config, but we'll add it to the file generation

        // Allow timeouts
        let allow_timeout = Confirm::with_theme(&self.theme)
            .with_prompt("Allow URLs that timeout to pass validation")
            .default(config.allow_timeout.unwrap_or(false))
            .interact()?;
        config.allow_timeout = Some(allow_timeout);

        // Retry attempts
        let retry_attempts: u8 = Input::with_theme(&self.theme)
            .with_prompt("Number of retry attempts for failed requests")
            .default(config.retry_attempts.unwrap_or(0))
            .validate_with(Self::validate_retry_attempts)
            .interact()?;
        config.retry_attempts = Some(retry_attempts);

        if retry_attempts > 0 {
            let retry_delay: u64 = Input::with_theme(&self.theme)
                .with_prompt("Delay between retries (milliseconds)")
                .default(config.retry_delay.unwrap_or(1000))
                .interact()?;
            config.retry_delay = Some(retry_delay);
        }

        // Rate limiting
        let rate_limit: u64 = Input::with_theme(&self.theme)
            .with_prompt("Delay between requests (milliseconds, 0 for no limit)")
            .default(config.rate_limit_delay.unwrap_or(0))
            .interact()?;
        if rate_limit > 0 {
            config.rate_limit_delay = Some(rate_limit);
        }

        // Failure threshold
        let use_threshold = Confirm::with_theme(&self.theme)
            .with_prompt("Set a failure threshold (allow some % of URLs to fail)")
            .default(config.failure_threshold.is_some())
            .interact()?;

        if use_threshold {
            let threshold: f64 = Input::with_theme(&self.theme)
                .with_prompt("Failure threshold percentage (0-100)")
                .default(config.failure_threshold.unwrap_or(0.0))
                .validate_with(Self::validate_failure_threshold)
                .interact()?;
            config.failure_threshold = Some(threshold);
        }

        Ok(config)
    }

    /// Validation function for thread count
    fn validate_thread_count(input: &usize) -> Result<(), &'static str> {
        if *input > 0 && *input <= 100 {
            Ok(())
        } else {
            Err("Must be between 1 and 100")
        }
    }

    /// Validation function for retry attempts
    fn validate_retry_attempts(input: &u8) -> Result<(), &'static str> {
        if *input <= 10 {
            Ok(())
        } else {
            Err("Maximum 10 retries")
        }
    }

    /// Validation function for failure threshold
    fn validate_failure_threshold(input: &f64) -> Result<(), &'static str> {
        if *input >= 0.0 && *input <= 100.0 {
            Ok(())
        } else {
            Err("Must be between 0 and 100")
        }
    }

    /// Configure URL filters (allowlists and exclusion patterns)
    fn configure_url_filters(&self, mut config: Config) -> WizardResult<Config> {
        println!("\n{}", colorize("🎯 URL Filtering", Colors::BRIGHT_WHITE));

        // Allowlist
        let setup_allowlist = Confirm::with_theme(&self.theme)
            .with_prompt("Set up URL allowlist (URLs containing these patterns will always pass)")
            .default(false)
            .interact()?;

        if setup_allowlist {
            let allowlist = self.collect_url_patterns("URL patterns")?;
            if !allowlist.is_empty() {
                config.allowlist = Some(allowlist);
            }
        }

        // Exclusion patterns
        let setup_exclusions = Confirm::with_theme(&self.theme)
            .with_prompt("Set up URL exclusion patterns (regex patterns to skip)")
            .default(false)
            .interact()?;

        if setup_exclusions {
            let exclusions = self.collect_url_patterns("regex patterns")?;
            if !exclusions.is_empty() {
                config.exclude_patterns = Some(exclusions);
            }
        }

        // Allowed status codes
        let setup_status_codes = Confirm::with_theme(&self.theme)
            .with_prompt("Allow specific HTTP status codes (e.g., 403 for restricted content)")
            .default(config.allowed_status_codes.is_some())
            .interact()?;

        if setup_status_codes {
            config.allowed_status_codes = self.collect_status_codes(&config)?;
        }

        Ok(config)
    }

    /// Collect URL patterns from user input
    fn collect_url_patterns(&self, pattern_type: &str) -> WizardResult<Vec<String>> {
        println!(
            "{}",
            colorize(
                &format!(
                    "Enter {} (one per line, empty line to finish):",
                    pattern_type
                ),
                Colors::DIM
            )
        );
        let mut patterns = Vec::new();

        loop {
            let pattern: String = Input::with_theme(&self.theme)
                .with_prompt(pattern_type.trim_end_matches('s')) // Remove plural
                .allow_empty(true)
                .interact()?;

            if pattern.is_empty() {
                break;
            }
            patterns.push(pattern);
        }

        Ok(patterns)
    }

    /// Collect and parse status codes
    fn collect_status_codes(&self, config: &Config) -> WizardResult<Option<Vec<u16>>> {
        let default_codes = config
            .allowed_status_codes
            .as_ref()
            .map(|codes| {
                codes
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();

        let codes_input: String = Input::with_theme(&self.theme)
            .with_prompt("Allowed status codes (comma-separated, e.g., 403,429,503)")
            .default(default_codes)
            .interact()?;

        if codes_input.is_empty() {
            return Ok(None);
        }

        let codes: Result<Vec<u16>, _> = codes_input.split(',').map(|s| s.trim().parse()).collect();

        match codes {
            Ok(codes) => Ok(Some(codes)),
            Err(_) => {
                println!(
                    "{}",
                    colorize("Invalid status codes, skipping...", Colors::YELLOW)
                );
                Ok(None)
            }
        }
    }
}

/// Configuration file generator
struct ConfigFileGenerator<'a> {
    config: &'a Config,
    file_types: &'a [&'a str],
}

impl<'a> ConfigFileGenerator<'a> {
    /// Create a new config file generator
    fn new(config: &'a Config, file_types: &'a [&'a str]) -> Self {
        Self { config, file_types }
    }

    /// Generate the configuration file content
    fn generate(&self) -> WizardResult<String> {
        let mut content = String::new();

        content.push_str("# urlsup configuration file\n");
        content.push_str("# Generated by the configuration wizard\n\n");

        self.add_basic_settings(&mut content);
        self.add_file_types(&mut content);

        content.push('\n');

        self.add_performance_settings(&mut content);
        self.add_filtering_settings(&mut content);
        self.add_quality_settings(&mut content);
        self.add_output_settings(&mut content);

        Ok(content)
    }

    /// Add basic settings section
    fn add_basic_settings(&self, content: &mut String) {
        content.push_str("# Basic settings\n");

        if let Some(timeout) = self.config.timeout {
            content.push_str(&format!("timeout = {}\n", timeout));
        }
        if let Some(allow_timeout) = self.config.allow_timeout {
            content.push_str(&format!("allow_timeout = {}\n", allow_timeout));
        }
    }

    /// Add file types section
    fn add_file_types(&self, content: &mut String) {
        if !self.file_types.is_empty() {
            content.push_str(&format!("file_types = {:?}\n", self.file_types));
        }
    }

    /// Add performance and retry settings
    fn add_performance_settings(&self, content: &mut String) {
        if self.config.retry_attempts.is_some()
            || self.config.retry_delay.is_some()
            || self.config.rate_limit_delay.is_some()
        {
            content.push_str("# Performance and retry settings\n");

            if let Some(retry) = self.config.retry_attempts {
                content.push_str(&format!("retry_attempts = {}\n", retry));
            }
            if let Some(delay) = self.config.retry_delay {
                content.push_str(&format!("retry_delay = {}\n", delay));
            }
            if let Some(rate_limit) = self.config.rate_limit_delay {
                content.push_str(&format!("rate_limit_delay = {}\n", rate_limit));
            }
            content.push('\n');
        }
    }

    /// Add filtering settings
    fn add_filtering_settings(&self, content: &mut String) {
        if self.config.allowlist.is_some()
            || self.config.exclude_patterns.is_some()
            || self.config.allowed_status_codes.is_some()
        {
            content.push_str("# URL filtering\n");

            if let Some(ref allowlist) = self.config.allowlist {
                content.push_str(&format!("allowlist = {:?}\n", allowlist));
            }
            if let Some(ref patterns) = self.config.exclude_patterns {
                content.push_str(&format!("exclude_patterns = {:?}\n", patterns));
            }
            if let Some(ref codes) = self.config.allowed_status_codes {
                content.push_str(&format!("allowed_status_codes = {:?}\n", codes));
            }
            content.push('\n');
        }
    }

    /// Add quality settings
    fn add_quality_settings(&self, content: &mut String) {
        if let Some(threshold) = self.config.failure_threshold {
            content.push_str("# Quality settings\n");
            content.push_str(&format!("failure_threshold = {:.1}\n", threshold));
            content.push('\n');
        }
    }

    /// Add output settings
    fn add_output_settings(&self, content: &mut String) {
        if let Some(ref format) = self.config.output_format {
            content.push_str("# Output settings\n");
            content.push_str(&format!("output_format = \"{}\"\n", format));
            content.push('\n');
        }
    }
}

/// Usage examples display helper
struct UsageExamples<'a> {
    file_types: &'a [&'a str],
}

impl<'a> UsageExamples<'a> {
    /// Create new usage examples helper
    fn new(file_types: &'a [&'a str]) -> Self {
        Self { file_types }
    }

    /// Display usage examples
    fn display(&self) {
        println!("\n{}", colorize("📚 Usage Examples", Colors::BRIGHT_WHITE));

        self.show_basic_usage();
        self.show_file_type_usage();
        self.show_advanced_usage();
    }

    /// Show basic usage examples
    fn show_basic_usage(&self) {
        println!("\n{}", colorize("Basic usage:", Colors::CYAN));
        println!("  {}", colorize("urlsup README.md", Colors::WHITE));
    }

    /// Show file type specific usage
    fn show_file_type_usage(&self) {
        if self.file_types.len() > 1 {
            let extensions = self.file_types.join(",");
            println!(
                "\n{}",
                colorize("Check all configured file types:", Colors::CYAN)
            );
            println!(
                "  {}",
                colorize(
                    &format!("urlsup --recursive --include {} .", extensions),
                    Colors::WHITE
                )
            );
        }
    }

    /// Show advanced usage examples
    fn show_advanced_usage(&self) {
        let examples = [
            ("With custom options:", "urlsup --verbose README.md"),
            ("JSON output for automation:", "urlsup --format json docs/"),
            (
                "Performance analysis:",
                "urlsup --show-performance --recursive docs/",
            ),
        ];

        for (description, command) in &examples {
            println!("\n{}", colorize(description, Colors::CYAN));
            println!("  {}", colorize(command, Colors::WHITE));
        }
    }
}

/// Run the interactive configuration wizard (public API)
pub fn run_configuration_wizard() -> Result<(), Box<dyn std::error::Error>> {
    ConfigurationWizard::new()
        .run()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_templates_are_valid() {
        let templates = get_project_templates();

        assert!(!templates.is_empty());

        for template in templates {
            assert!(!template.name.is_empty());
            assert!(!template.description.is_empty());
            assert!(!template.file_types.is_empty());

            // Validate timeouts are reasonable
            if let Some(timeout) = template.config.timeout {
                assert!(timeout > 0 && timeout <= 300);
            }

            // Validate retry attempts are reasonable
            if let Some(retries) = template.config.retry_attempts {
                assert!(retries <= 10);
            }

            // Validate failure threshold is percentage
            if let Some(threshold) = template.config.failure_threshold {
                assert!((0.0..=100.0).contains(&threshold));
            }
        }
    }

    #[test]
    fn test_generate_config_file_basic() {
        let config = Config {
            timeout: Some(30),
            allow_timeout: Some(true),
            ..Config::default()
        };
        let file_types = vec!["md", "txt"];

        let generator = ConfigFileGenerator::new(&config, &file_types);
        let content = generator.generate().unwrap();

        assert!(content.contains("timeout = 30"));
        assert!(content.contains("allow_timeout = true"));
        assert!(content.contains(r#"file_types = ["md", "txt"]"#));
    }

    #[test]
    fn test_generate_config_file_advanced() {
        let config = Config {
            timeout: Some(45),
            retry_attempts: Some(3),
            retry_delay: Some(2000),
            allowlist: Some(vec!["example.com".to_string(), "localhost".to_string()]),
            allowed_status_codes: Some(vec![403, 404, 429]),
            failure_threshold: Some(10.5),
            ..Config::default()
        };
        let file_types = vec!["md", "html"];

        let generator = ConfigFileGenerator::new(&config, &file_types);
        let content = generator.generate().unwrap();

        assert!(content.contains("timeout = 45"));
        assert!(content.contains("retry_attempts = 3"));
        assert!(content.contains("retry_delay = 2000"));
        assert!(content.contains(r#"allowlist = ["example.com", "localhost"]"#));
        assert!(content.contains("allowed_status_codes = [403, 404, 429]"));
        assert!(content.contains("failure_threshold = 10.5"));
    }

    #[test]
    fn test_generate_config_file_minimal() {
        let config = Config::default();
        let file_types = vec!["md"];

        let generator = ConfigFileGenerator::new(&config, &file_types);
        let content = generator.generate().unwrap();

        assert!(content.contains(r#"file_types = ["md"]"#));
        assert!(content.contains("# urlsup configuration file"));
    }

    #[test]
    fn test_wizard_error_display() {
        let io_err = WizardError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_err.to_string().contains("IO error"));

        let dialog_err = WizardError::Dialog(dialoguer::Error::IO(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid input",
        )));
        assert!(dialog_err.to_string().contains("Dialog error"));
    }

    #[test]
    fn test_project_template_display() {
        let templates = get_project_templates();
        let template = &templates[0];
        let display_str = template.to_string();
        assert!(display_str.contains(template.name));
        assert!(display_str.contains(template.description));
    }

    #[test]
    fn test_usage_examples() {
        let file_types = vec!["md", "txt"];
        let examples = UsageExamples::new(&file_types);
        // Just ensure it doesn't panic
        examples.display();
    }
}
