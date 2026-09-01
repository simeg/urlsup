//! Application-wide constants to avoid magic values throughout the codebase.
//!
//! This module centralizes all magic strings, numbers, and other literal values
//! used across the application, making them easier to maintain and modify.

/// Output format constants
pub mod output_formats {
    /// Text output format - colorful, emoji-enhanced output with grouping
    pub const TEXT: &str = "text";
    /// JSON output format - structured output for automation
    pub const JSON: &str = "json";
    /// Minimal output format - plain text without colors or emojis
    pub const MINIMAL: &str = "minimal";

    /// Default output format
    pub const DEFAULT: &str = TEXT;

    /// All valid output formats
    pub const ALL: [&str; 3] = [TEXT, JSON, MINIMAL];
}

/// Timeout and duration constants
pub mod timeouts {
    /// Default connection timeout in seconds
    pub const DEFAULT_TIMEOUT_SECONDS: u64 = 5;
    /// Maximum reasonable timeout in seconds (1 hour)
    pub const MAX_TIMEOUT_SECONDS: u64 = 3600;
    /// Default retry delay in milliseconds
    pub const DEFAULT_RETRY_DELAY_MS: u64 = 1000;
    /// Default rate limit delay in milliseconds (no delay)
    pub const DEFAULT_RATE_LIMIT_MS: u64 = 0;
}

/// Error message constants
pub mod error_messages {
    /// Timeout error message from reqwest
    pub const OPERATION_TIMED_OUT: &str = "operation timed out";
}

/// File processing constants
pub mod files {
    /// Default capacity hint for URL matches per file
    pub const DEFAULT_URL_CAPACITY_PER_FILE: usize = 20;
    /// Estimated URLs per matched line
    pub const ESTIMATED_URLS_PER_MATCH: usize = 2;
}
