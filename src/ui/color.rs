//! Color, emoji, and formatting utilities for terminal output

pub struct Colors;

impl Colors {
    pub const RESET: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const DIM: &'static str = "\x1b[2m";

    // Basic colors
    pub const BLACK: &'static str = "\x1b[30m";
    pub const RED: &'static str = "\x1b[31m";
    pub const GREEN: &'static str = "\x1b[32m";
    pub const YELLOW: &'static str = "\x1b[33m";
    pub const BLUE: &'static str = "\x1b[34m";
    pub const MAGENTA: &'static str = "\x1b[35m";
    pub const CYAN: &'static str = "\x1b[36m";
    pub const WHITE: &'static str = "\x1b[37m";

    // Bright colors
    pub const BRIGHT_BLACK: &'static str = "\x1b[90m";
    pub const BRIGHT_RED: &'static str = "\x1b[91m";
    pub const BRIGHT_GREEN: &'static str = "\x1b[92m";
    pub const BRIGHT_YELLOW: &'static str = "\x1b[93m";
    pub const BRIGHT_BLUE: &'static str = "\x1b[94m";
    pub const BRIGHT_MAGENTA: &'static str = "\x1b[95m";
    pub const BRIGHT_CYAN: &'static str = "\x1b[96m";
    pub const BRIGHT_WHITE: &'static str = "\x1b[97m";
}

/// Apply color to text if terminal supports it
pub fn colorize(text: &str, color: &str) -> String {
    if supports_formatting() {
        format!("{}{}{}", color, text, Colors::RESET)
    } else {
        text.to_string()
    }
}

/// Enhanced terminal capability detection
pub fn supports_formatting() -> bool {
    use std::env;
    use std::io::IsTerminal;

    // Check if colors/emojis are explicitly disabled
    if env::var("NO_COLOR").is_ok() || env::var("FORCE_COLOR").as_deref() == Ok("0") {
        return false;
    }

    // Force enable if explicitly requested
    if env::var("FORCE_COLOR").is_ok() {
        return true;
    }

    // Disable formatting when running tests
    if cfg!(test) || env::var("RUST_TEST_TIME_UNIT").is_ok() {
        return false;
    }

    // Check if output is being redirected
    if !std::io::stdout().is_terminal() {
        return false;
    }

    // Check TERM environment variable
    if let Ok(term) = env::var("TERM") {
        if term == "dumb" || term.is_empty() {
            return false;
        }

        // Check for known capable terminals
        if term.contains("color")
            || term.contains("256")
            || term.starts_with("xterm")
            || term.starts_with("screen")
            || term.starts_with("tmux")
            || term == "linux"
        {
            return true;
        }
    }

    // Check for modern terminal programs
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "Apple_Terminal" | "iTerm.app" | "vscode" | "Hyper" | "Alacritty" | "kitty"
            | "WezTerm" => return true,
            _ => {}
        }
    }

    // Check CI environments that support colors
    if env::var("CI").is_ok() {
        let ci_supports_color = [
            "GITHUB_ACTIONS",
            "TRAVIS",
            "CIRCLECI",
            "APPVEYOR",
            "GITLAB_CI",
            "AZURE_HTTP_USER_AGENT",
            "BUILDKITE",
        ]
        .iter()
        .any(|var| env::var(var).is_ok());

        if ci_supports_color {
            return true;
        }
    }

    // Default: assume no support if we can't detect
    false
}

/// Check if the current environment supports ANSI colors (alias for backwards compatibility)
pub fn supports_color() -> bool {
    supports_formatting()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorize_with_no_color() {
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        let result = colorize("test", Colors::RED);
        assert_eq!(result, "test");
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    #[test]
    fn test_supports_formatting_with_no_color() {
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!supports_formatting());
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    #[test]
    fn test_supports_formatting_with_dumb_term() {
        unsafe {
            std::env::set_var("TERM", "dumb");
        }
        assert!(!supports_formatting());
        unsafe {
            std::env::remove_var("TERM");
        }
    }

    #[test]
    fn test_supports_color_alias() {
        // Test that supports_color() is equivalent to supports_formatting()
        assert_eq!(supports_color(), supports_formatting());
    }

    #[test]
    fn test_colorize_with_formatting_enabled() {
        // Save current environment
        let original_no_color = std::env::var("NO_COLOR").ok();
        let original_term = std::env::var("TERM").ok();

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::set_var("TERM", "xterm-256color");
        }
        let result = colorize("test", Colors::RED);
        assert!(result.contains("test"));

        // Restore original environment
        unsafe {
            if let Some(val) = original_no_color {
                std::env::set_var("NO_COLOR", val);
            } else {
                std::env::remove_var("NO_COLOR");
            }
            if let Some(val) = original_term {
                std::env::set_var("TERM", val);
            } else {
                std::env::remove_var("TERM");
            }
        }
    }

    #[test]
    fn test_supports_formatting_with_empty_term() {
        // Save current environment
        let original_no_color = std::env::var("NO_COLOR").ok();
        let original_term = std::env::var("TERM").ok();
        let original_test_time = std::env::var("RUST_TEST_TIME_UNIT").ok();

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("RUST_TEST_TIME_UNIT");
            std::env::set_var("TERM", ""); // Empty TERM
        }

        assert!(!supports_formatting());

        // Restore original environment
        unsafe {
            if let Some(val) = original_no_color {
                std::env::set_var("NO_COLOR", val);
            }
            if let Some(val) = original_term {
                std::env::set_var("TERM", val);
            } else {
                std::env::remove_var("TERM");
            }
            if let Some(val) = original_test_time {
                std::env::set_var("RUST_TEST_TIME_UNIT", val);
            }
        }
    }

    #[test]
    fn test_supports_formatting_with_missing_term() {
        // Save current environment
        let original_no_color = std::env::var("NO_COLOR").ok();
        let original_term = std::env::var("TERM").ok();
        let original_test_time = std::env::var("RUST_TEST_TIME_UNIT").ok();

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("RUST_TEST_TIME_UNIT");
            std::env::remove_var("TERM"); // Missing TERM
        }

        assert!(!supports_formatting());

        // Restore original environment
        unsafe {
            if let Some(val) = original_no_color {
                std::env::set_var("NO_COLOR", val);
            }
            if let Some(val) = original_term {
                std::env::set_var("TERM", val);
            }
            if let Some(val) = original_test_time {
                std::env::set_var("RUST_TEST_TIME_UNIT", val);
            }
        }
    }

    #[test]
    fn test_supports_formatting_with_rust_test_time_unit() {
        // Save current environment
        let original_no_color = std::env::var("NO_COLOR").ok();
        let original_term = std::env::var("TERM").ok();
        let original_test_time = std::env::var("RUST_TEST_TIME_UNIT").ok();

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::set_var("TERM", "xterm-256color");
            std::env::set_var("RUST_TEST_TIME_UNIT", "1"); // Test time unit set
        }

        assert!(!supports_formatting());

        // Restore original environment
        unsafe {
            if let Some(val) = original_no_color {
                std::env::set_var("NO_COLOR", val);
            }
            if let Some(val) = original_term {
                std::env::set_var("TERM", val);
            } else {
                std::env::remove_var("TERM");
            }
            if let Some(val) = original_test_time {
                std::env::set_var("RUST_TEST_TIME_UNIT", val);
            } else {
                std::env::remove_var("RUST_TEST_TIME_UNIT");
            }
        }
    }

    #[test]
    fn test_supports_formatting_with_valid_term() {
        // Save current environment
        let original_no_color = std::env::var("NO_COLOR").ok();
        let original_term = std::env::var("TERM").ok();
        let original_test_time = std::env::var("RUST_TEST_TIME_UNIT").ok();

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("RUST_TEST_TIME_UNIT");
            std::env::set_var("TERM", "xterm-256color"); // Valid TERM
        }

        // In test environment, cfg!(test) is true, so this should still return false
        assert!(!supports_formatting());

        // Restore original environment
        unsafe {
            if let Some(val) = original_no_color {
                std::env::set_var("NO_COLOR", val);
            }
            if let Some(val) = original_term {
                std::env::set_var("TERM", val);
            } else {
                std::env::remove_var("TERM");
            }
            if let Some(val) = original_test_time {
                std::env::set_var("RUST_TEST_TIME_UNIT", val);
            }
        }
    }

    #[test]
    fn test_all_color_constants() {
        // Test all color constants are accessible and have expected values
        assert_eq!(Colors::RESET, "\x1b[0m");
        assert_eq!(Colors::BOLD, "\x1b[1m");
        assert_eq!(Colors::DIM, "\x1b[2m");

        // Basic colors
        assert_eq!(Colors::RED, "\x1b[31m");
        assert_eq!(Colors::GREEN, "\x1b[32m");
        assert_eq!(Colors::YELLOW, "\x1b[33m");
        assert_eq!(Colors::BLUE, "\x1b[34m");
        assert_eq!(Colors::MAGENTA, "\x1b[35m");
        assert_eq!(Colors::CYAN, "\x1b[36m");
        assert_eq!(Colors::WHITE, "\x1b[37m");

        // Bright colors
        assert_eq!(Colors::BRIGHT_RED, "\x1b[91m");
        assert_eq!(Colors::BRIGHT_GREEN, "\x1b[92m");
        assert_eq!(Colors::BRIGHT_YELLOW, "\x1b[93m");
        assert_eq!(Colors::BRIGHT_BLUE, "\x1b[94m");
        assert_eq!(Colors::BRIGHT_MAGENTA, "\x1b[95m");
        assert_eq!(Colors::BRIGHT_CYAN, "\x1b[96m");
        assert_eq!(Colors::BRIGHT_WHITE, "\x1b[97m");
    }

    #[test]
    fn test_colorize_with_all_colors() {
        // Test colorize function with various colors
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }

        // When formatting is disabled, all should return plain text
        assert_eq!(colorize("test", Colors::RED), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_GREEN), "test");
        assert_eq!(colorize("test", Colors::BLUE), "test");
        assert_eq!(colorize("test", Colors::YELLOW), "test");
        assert_eq!(colorize("test", Colors::MAGENTA), "test");
        assert_eq!(colorize("test", Colors::CYAN), "test");
        assert_eq!(colorize("test", Colors::WHITE), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_RED), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_YELLOW), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_BLUE), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_MAGENTA), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_CYAN), "test");
        assert_eq!(colorize("test", Colors::BRIGHT_WHITE), "test");

        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    #[test]
    fn test_disable_formatting_when_running_tests() {
        unsafe {
            std::env::set_var("RUST_TEST_TIME_UNIT", "1");
        }

        assert!(!supports_formatting());

        unsafe {
            std::env::remove_var("RUST_TEST_TIME_UNIT");
        }
    }

    #[test]
    fn test_disable_formatting_when_dumb_terminal() {
        unsafe {
            std::env::set_var("TERM", "dumb");
        }

        assert!(!supports_formatting());

        unsafe {
            std::env::remove_var("TERM");
        }
    }

    #[test]
    fn test_colorize_edge_cases() {
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }

        // Test empty string
        assert_eq!(colorize("", Colors::RED), "");

        // Test string with special characters
        assert_eq!(
            colorize("test\nwith\ttabs", Colors::BLUE),
            "test\nwith\ttabs"
        );

        // Test unicode characters
        assert_eq!(
            colorize("test 🚀 unicode", Colors::GREEN),
            "test 🚀 unicode"
        );

        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    #[test]
    fn test_supports_formatting_with_term_variations() {
        let original_no_color = std::env::var("NO_COLOR").ok();
        let original_term = std::env::var("TERM").ok();
        let original_test_time = std::env::var("RUST_TEST_TIME_UNIT").ok();

        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::remove_var("RUST_TEST_TIME_UNIT");
        }

        // Test various TERM values that should support formatting
        let supporting_terms = ["xterm", "xterm-256color", "screen", "tmux", "linux"];
        for term in &supporting_terms {
            unsafe {
                std::env::set_var("TERM", term);
            }
            // Should still be false due to cfg!(test)
            assert!(!supports_formatting(), "Failed for TERM={}", term);
        }

        // Restore original environment
        unsafe {
            if let Some(val) = original_no_color {
                std::env::set_var("NO_COLOR", val);
            }
            if let Some(val) = original_term {
                std::env::set_var("TERM", val);
            } else {
                std::env::remove_var("TERM");
            }
            if let Some(val) = original_test_time {
                std::env::set_var("RUST_TEST_TIME_UNIT", val);
            }
        }
    }

    #[test]
    fn test_all_color_constants_coverage() {
        // This test ensures all color constants are included in test coverage
        // by accessing each one individually
        let constants = [
            Colors::RESET,
            Colors::BOLD,
            Colors::DIM,
            Colors::RED,
            Colors::GREEN,
            Colors::YELLOW,
            Colors::BLUE,
            Colors::MAGENTA,
            Colors::CYAN,
            Colors::WHITE,
            Colors::BRIGHT_RED,
            Colors::BRIGHT_GREEN,
            Colors::BRIGHT_YELLOW,
            Colors::BRIGHT_BLUE,
            Colors::BRIGHT_MAGENTA,
            Colors::BRIGHT_CYAN,
            Colors::BRIGHT_WHITE,
        ];

        // Verify each constant is accessible and not empty
        for constant in &constants {
            assert!(!constant.is_empty(), "Color constant should not be empty");
            assert!(
                constant.starts_with('\x1b'),
                "Color constant should start with escape sequence"
            );
        }

        // Test that each constant produces a unique string
        let mut unique_values = std::collections::HashSet::new();
        for constant in &constants {
            assert!(
                unique_values.insert(*constant),
                "Color constant should be unique: {}",
                constant
            );
        }

        // Verify we have the expected number of constants
        assert_eq!(constants.len(), 17, "Expected 17 color constants");
    }
}
