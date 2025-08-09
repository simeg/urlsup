//! urlsup - A fast, reliable URL validation tool
//!
//! This library provides functionality for finding URLs in files and
//! validating them via HTTP requests. It's designed to be fast, concurrent,
//! and suitable for CI/CD pipelines.

pub mod config;
pub mod core;
pub mod discovery;
pub mod reporting;
pub mod ui;
pub mod validation;

// Re-export commonly used types for convenience
pub use config::{CliConfig, Config};
pub use core::{UrlLocation, UrlLocationBuilder, UrlLocationError};
pub use discovery::{Finder, UrlFinder};
pub use reporting::{DashboardData, HtmlDashboard, PerformanceProfiler};
pub use ui::{Cli, Commands, DisplayMetadata, ProgressReporter, cli_to_config};
pub use validation::{ValidateUrls, ValidationResult, Validator};
