/// Application-wide constants to avoid magic values throughout the codebase.
///
/// This module centralizes all magic strings, numbers, and other literal values
/// used across the application, making them easier to maintain and modify.
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

/// HTTP status code constants
pub mod http_status {
    /// HTTP 200 OK - successful response
    pub const OK: u16 = 200;
    /// HTTP 404 Not Found - resource not found
    pub const NOT_FOUND: u16 = 404;
    /// HTTP 500 Internal Server Error - server error
    pub const INTERNAL_SERVER_ERROR: u16 = 500;
    /// HTTP 403 Forbidden - access forbidden
    pub const FORBIDDEN: u16 = 403;
    /// HTTP 301 Moved Permanently - permanent redirect
    pub const MOVED_PERMANENTLY: u16 = 301;
    /// HTTP 302 Found - temporary redirect
    pub const FOUND: u16 = 302;
    /// HTTP 502 Bad Gateway - bad gateway error
    pub const BAD_GATEWAY: u16 = 502;
}

/// Timeout and duration constants
pub mod timeouts {
    /// Default connection timeout in seconds
    pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;
    /// Maximum reasonable timeout in seconds (1 hour)
    pub const MAX_TIMEOUT_SECONDS: u64 = 3600;
    /// Minimum timeout in seconds
    pub const MIN_TIMEOUT_SECONDS: u64 = 1;
    /// Default retry delay in milliseconds
    pub const DEFAULT_RETRY_DELAY_MS: u64 = 1000;
    /// Default rate limit delay in milliseconds (no delay)
    pub const DEFAULT_RATE_LIMIT_MS: u64 = 0;
}

/// Default configuration values
pub mod defaults {
    /// Default number of retry attempts
    pub const RETRY_ATTEMPTS: u32 = 0;
    /// Default concurrency (will be set to CPU cores at runtime)
    pub const CONCURRENCY_AUTO: u32 = 0;
    /// Default failure threshold percentage (no threshold)
    pub const NO_FAILURE_THRESHOLD: Option<f64> = None;
}

/// Validation constants
pub mod validation {
    /// Minimum line number (1-indexed)
    pub const MIN_LINE_NUMBER: u64 = 1;
    /// Maximum percentage value
    pub const MAX_PERCENTAGE: f64 = 100.0;
    /// Minimum percentage value
    pub const MIN_PERCENTAGE: f64 = 0.0;
}

/// Error message constants
pub mod error_messages {
    /// Timeout error message from reqwest
    pub const OPERATION_TIMED_OUT: &str = "operation timed out";
    /// Connection timeout error
    pub const CONNECTION_TIMEOUT: &str = "Connection timeout";
    /// Unknown error fallback
    pub const UNKNOWN_ERROR: &str = "Unknown error";
}

/// File processing constants
pub mod files {
    /// Default capacity hint for URL matches per file
    pub const DEFAULT_URL_CAPACITY_PER_FILE: usize = 20;
    /// Estimated URLs per matched line
    pub const ESTIMATED_URLS_PER_MATCH: usize = 2;
    /// Maximum files to display in config info before truncating
    pub const MAX_FILES_TO_DISPLAY: usize = 10;
}

/// Display and formatting constants
pub mod display {
    /// Emoji for success status
    pub const SUCCESS_EMOJI: &str = "✅";
    /// Emoji for warning status
    pub const WARNING_EMOJI: &str = "⚠️";
    /// Emoji for error status
    pub const ERROR_EMOJI: &str = "❌";
    /// Emoji for network/connection errors
    pub const NETWORK_ERROR_EMOJI: &str = "🔌";
    /// Emoji for server errors
    pub const SERVER_ERROR_EMOJI: &str = "💥";
    /// Emoji for client errors
    pub const CLIENT_ERROR_EMOJI: &str = "🚫";
    /// Emoji for redirect issues
    pub const REDIRECT_EMOJI: &str = "🔄";
    /// Emoji for other issues
    pub const OTHER_EMOJI: &str = "❓";
    /// Emoji for file information
    pub const FILE_EMOJI: &str = "📁";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_formats_constants() {
        assert_eq!(output_formats::TEXT, "text");
        assert_eq!(output_formats::JSON, "json");
        assert_eq!(output_formats::MINIMAL, "minimal");
        assert_eq!(output_formats::DEFAULT, "text");
        assert_eq!(output_formats::ALL.len(), 3);
    }

    #[test]
    fn test_http_status_constants() {
        assert_eq!(http_status::OK, 200);
        assert_eq!(http_status::NOT_FOUND, 404);
        assert_eq!(http_status::INTERNAL_SERVER_ERROR, 500);
    }

    #[test]
    fn test_timeout_constants() {
        assert_eq!(timeouts::DEFAULT_TIMEOUT_SECONDS, 30);
        assert_eq!(timeouts::MAX_TIMEOUT_SECONDS, 3600);
        assert_eq!(timeouts::MIN_TIMEOUT_SECONDS, 1);
    }

    #[test]
    fn test_validation_constants() {
        assert_eq!(validation::MIN_LINE_NUMBER, 1);
        assert_eq!(validation::MAX_PERCENTAGE, 100.0);
        assert_eq!(validation::MIN_PERCENTAGE, 0.0);
    }

    #[test]
    fn test_error_message_constants() {
        assert_eq!(error_messages::OPERATION_TIMED_OUT, "operation timed out");
        assert_eq!(error_messages::CONNECTION_TIMEOUT, "Connection timeout");
        assert_eq!(error_messages::UNKNOWN_ERROR, "Unknown error");
    }
}
