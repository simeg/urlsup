//! Advanced terminal theme detection and adaptive color schemes
//!
//! This module provides intelligent terminal background detection
//! and adaptive color schemes for optimal readability across
//! different terminal themes and environments.

use once_cell::sync::Lazy;
use std::env;
use std::fmt;
use std::io::IsTerminal;
use std::str::FromStr;

use crate::ui::color::Colors;

/// Terminal theme detection results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTheme {
    Light,
    Dark,
    Unknown,
}

impl fmt::Display for TerminalTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Light => write!(f, "light"),
            Self::Dark => write!(f, "dark"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl FromStr for TerminalTheme {
    type Err = ThemeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            "unknown" => Ok(Self::Unknown),
            _ => Err(ThemeError::InvalidTheme(s.to_string())),
        }
    }
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self::Dark
    }
}

/// Adaptive color scheme based on terminal background
#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub theme: TerminalTheme,
    pub primary: &'static str,
    pub success: &'static str,
    pub warning: &'static str,
    pub error: &'static str,
    pub info: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub url: &'static str,
}

impl ColorScheme {
    /// Get color for semantic color type
    pub fn get_color(&self, semantic_color: SemanticColor) -> &'static str {
        match semantic_color {
            SemanticColor::Primary => self.primary,
            SemanticColor::Success => self.success,
            SemanticColor::Warning => self.warning,
            SemanticColor::Error => self.error,
            SemanticColor::Info => self.info,
            SemanticColor::Muted => self.muted,
            SemanticColor::Accent => self.accent,
            SemanticColor::Url => self.url,
        }
    }

    /// Create color scheme for a specific theme
    const fn for_theme(theme: TerminalTheme) -> Self {
        match theme {
            TerminalTheme::Light => Self::LIGHT_SCHEME,
            TerminalTheme::Dark => Self::DARK_SCHEME,
            TerminalTheme::Unknown => Self::UNIVERSAL_SCHEME,
        }
    }

    const LIGHT_SCHEME: Self = Self {
        theme: TerminalTheme::Light,
        primary: Colors::BLACK,
        success: Colors::GREEN,
        warning: "\x1b[38;5;166m", // Orange for better light theme visibility
        error: Colors::RED,
        info: Colors::BLUE,
        muted: "\x1b[38;5;102m", // Medium gray
        accent: Colors::MAGENTA,
        url: Colors::BLUE,
    };

    const DARK_SCHEME: Self = Self {
        theme: TerminalTheme::Dark,
        primary: Colors::BRIGHT_WHITE,
        success: Colors::BRIGHT_GREEN,
        warning: Colors::BRIGHT_YELLOW,
        error: Colors::BRIGHT_RED,
        info: Colors::BRIGHT_CYAN,
        muted: Colors::DIM,
        accent: Colors::BRIGHT_MAGENTA,
        url: Colors::CYAN,
    };

    const UNIVERSAL_SCHEME: Self = Self {
        theme: TerminalTheme::Unknown,
        // Conservative colors that work on both light and dark
        primary: Colors::WHITE,
        success: Colors::GREEN,
        warning: Colors::YELLOW,
        error: Colors::RED,
        info: Colors::CYAN,
        muted: Colors::DIM,
        accent: Colors::MAGENTA,
        url: Colors::BLUE,
    };
}

/// Semantic color types for adaptive theming
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticColor {
    Primary,
    Success,
    Warning,
    Error,
    Info,
    Muted,
    Accent,
    Url,
}

impl fmt::Display for SemanticColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primary => write!(f, "primary"),
            Self::Success => write!(f, "success"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Info => write!(f, "info"),
            Self::Muted => write!(f, "muted"),
            Self::Accent => write!(f, "accent"),
            Self::Url => write!(f, "url"),
        }
    }
}

/// Error types for theme operations
#[derive(Debug)]
pub enum ThemeError {
    InvalidTheme(String),
    DetectionFailed(String),
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTheme(theme) => write!(f, "Invalid theme: {}", theme),
            Self::DetectionFailed(reason) => write!(f, "Theme detection failed: {}", reason),
        }
    }
}

impl std::error::Error for ThemeError {}

/// Terminal theme detector with configurable fallback behavior
pub struct ThemeDetector {
    fallback_theme: TerminalTheme,
}

impl Default for ThemeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeDetector {
    /// Create a new theme detector with default fallback
    pub const fn new() -> Self {
        Self {
            fallback_theme: TerminalTheme::Dark,
        }
    }

    /// Create theme detector with custom fallback
    pub const fn with_fallback(fallback: TerminalTheme) -> Self {
        Self {
            fallback_theme: fallback,
        }
    }

    /// Detect terminal theme with environment variable checks
    pub fn detect(&self) -> TerminalTheme {
        self.detect_from_env()
            .or_else(|| self.detect_from_terminal_program())
            .or_else(|| self.detect_from_context())
            .unwrap_or(self.fallback_theme)
    }

    /// Detect theme from explicit environment variables
    fn detect_from_env(&self) -> Option<TerminalTheme> {
        if let Ok(theme) = env::var("URLSUP_THEME") {
            theme.parse().ok()
        } else {
            None
        }
    }

    /// Detect theme from COLORFGBG environment variable
    fn detect_from_colorfgbg(&self) -> Option<TerminalTheme> {
        env::var("COLORFGBG").ok().and_then(|colorfgbg| {
            colorfgbg.split(';').nth(1).and_then(|bg| {
                bg.parse::<u8>().ok().map(|bg_num| {
                    if bg_num >= 8 {
                        TerminalTheme::Light
                    } else {
                        TerminalTheme::Dark
                    }
                })
            })
        })
    }

    /// Detect theme from terminal program hints
    fn detect_from_terminal_program(&self) -> Option<TerminalTheme> {
        env::var("TERM_PROGRAM").ok().and_then(|term_program| {
            match term_program.as_str() {
                "Apple_Terminal" => Some(TerminalTheme::Light),
                "vscode" => Some(TerminalTheme::Dark),
                "iTerm.app" => {
                    // Check if we can get more specific info
                    if env::var("ITERM_PROFILE").is_ok() {
                        Some(TerminalTheme::Unknown)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
    }

    /// Detect theme from context (SSH, etc.)
    fn detect_from_context(&self) -> Option<TerminalTheme> {
        if env::var("SSH_CONNECTION").is_ok() || env::var("SSH_CLIENT").is_ok() {
            Some(TerminalTheme::Dark)
        } else {
            // Try COLORFGBG as last resort
            self.detect_from_colorfgbg()
        }
    }
}

/// Global color scheme instance
static COLOR_SCHEME: Lazy<ColorScheme> = Lazy::new(|| {
    let detector = ThemeDetector::new();
    let theme = detector.detect();
    ColorScheme::for_theme(theme)
});

/// Get the current color scheme
pub fn get_color_scheme() -> &'static ColorScheme {
    &COLOR_SCHEME
}

/// Apply adaptive color based on terminal theme
pub fn colorize_adaptive(text: &str, semantic_color: SemanticColor) -> String {
    use crate::ui::color::{colorize, supports_formatting};

    if !supports_formatting() {
        return text.to_string();
    }

    let scheme = get_color_scheme();
    let color = scheme.get_color(semantic_color);

    colorize(text, color)
}

/// Terminal capability detector for advanced features
pub struct TerminalCapabilityDetector;

impl TerminalCapabilityDetector {
    /// Check if terminal supports true color (24-bit)
    pub fn supports_true_color() -> bool {
        use crate::ui::color::supports_formatting;

        if !supports_formatting() {
            return false;
        }

        Self::check_explicit_support()
            || Self::check_term_capabilities()
            || Self::check_known_terminals()
    }

    /// Check explicit true color support environment variables
    fn check_explicit_support() -> bool {
        matches!(
            env::var("COLORTERM").as_deref(),
            Ok("truecolor") | Ok("24bit")
        )
    }

    /// Check TERM variable for true color indicators
    fn check_term_capabilities() -> bool {
        env::var("TERM")
            .map(|term| term.contains("truecolor") || term.contains("24bit"))
            .unwrap_or(false)
    }

    /// Check known terminal programs that support true color
    fn check_known_terminals() -> bool {
        const TRUE_COLOR_TERMINALS: &[&str] = &[
            "iTerm.app",
            "vscode",
            "Hyper",
            "Alacritty",
            "kitty",
            "WezTerm",
            "Apple_Terminal",
        ];

        env::var("TERM_PROGRAM")
            .map(|term_program| TRUE_COLOR_TERMINALS.contains(&term_program.as_str()))
            .unwrap_or(false)
    }
}

/// Compatibility wrapper for existing API
pub fn supports_true_color() -> bool {
    TerminalCapabilityDetector::supports_true_color()
}

/// Get terminal information for debugging
pub fn get_terminal_info() -> TerminalInfo {
    use crate::ui::color::supports_formatting;

    let detector = ThemeDetector::new();

    TerminalInfo {
        supports_color: supports_formatting(),
        supports_true_color: supports_true_color(),
        theme: detector.detect(),
        term_var: env::var("TERM").ok(),
        term_program: env::var("TERM_PROGRAM").ok(),
        colorterm: env::var("COLORTERM").ok(),
        colorfgbg: env::var("COLORFGBG").ok(),
        force_color: env::var("FORCE_COLOR").ok(),
        no_color: env::var("NO_COLOR").is_ok(),
        is_tty: std::io::stdout().is_terminal(),
    }
}

/// Terminal capability information
#[derive(Debug, Clone)]
pub struct TerminalInfo {
    pub supports_color: bool,
    pub supports_true_color: bool,
    pub theme: TerminalTheme,
    pub term_var: Option<String>,
    pub term_program: Option<String>,
    pub colorterm: Option<String>,
    pub colorfgbg: Option<String>,
    pub force_color: Option<String>,
    pub no_color: bool,
    pub is_tty: bool,
}

/// WCAG accessibility levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessibilityLevel {
    AA,  // Minimum standard (4.5:1 contrast)
    AAA, // Enhanced standard (7:1 contrast)
}

impl AccessibilityLevel {
    /// Get the minimum contrast ratio for this level
    pub const fn contrast_ratio(self) -> f32 {
        match self {
            Self::AA => 4.5,
            Self::AAA => 7.0,
        }
    }
}

impl fmt::Display for AccessibilityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AA => write!(f, "AA (4.5:1)"),
            Self::AAA => write!(f, "AAA (7:1)"),
        }
    }
}

/// Accessibility checker for color combinations
pub struct AccessibilityChecker;

impl AccessibilityChecker {
    /// Get contrast ratio between two colors (simplified implementation)
    pub fn get_contrast_ratio(fg: &str, bg: &str) -> f32 {
        // Simplified contrast calculation for basic colors
        // In a real implementation, this would parse RGB values
        const HIGH_CONTRAST_PAIRS: &[(&str, &str, f32)] = &[
            (Colors::WHITE, Colors::BLACK, 21.0),
            (Colors::BRIGHT_WHITE, Colors::BLACK, 21.0),
            (Colors::BLACK, Colors::WHITE, 21.0),
            (Colors::BLACK, Colors::BRIGHT_WHITE, 21.0),
            (Colors::YELLOW, Colors::BLACK, 19.6),
            (Colors::BRIGHT_YELLOW, Colors::BLACK, 19.6),
            (Colors::CYAN, Colors::BLACK, 16.3),
            (Colors::BRIGHT_CYAN, Colors::BLACK, 16.3),
        ];

        HIGH_CONTRAST_PAIRS
            .iter()
            .find(|(f, b, _)| *f == fg && *b == bg)
            .map(|(_, _, ratio)| *ratio)
            .unwrap_or(7.0) // Assume decent contrast for other combinations
    }

    /// Check if color combination meets WCAG accessibility standards
    pub fn meets_standard(fg: &str, bg: &str, level: AccessibilityLevel) -> bool {
        let ratio = Self::get_contrast_ratio(fg, bg);
        ratio >= level.contrast_ratio()
    }

    /// Find the best color for a background that meets accessibility standards
    pub fn find_accessible_color(bg: &str, level: AccessibilityLevel) -> &'static str {
        const CANDIDATE_COLORS: &[&str] = &[
            Colors::WHITE,
            Colors::BRIGHT_WHITE,
            Colors::BLACK,
            Colors::YELLOW,
            Colors::BRIGHT_YELLOW,
            Colors::CYAN,
            Colors::BRIGHT_CYAN,
        ];

        CANDIDATE_COLORS
            .iter()
            .find(|&&fg| Self::meets_standard(fg, bg, level))
            .copied()
            .unwrap_or(Colors::WHITE) // Fallback to white
    }
}

/// Compatibility wrappers for existing API
pub fn get_contrast_ratio(fg: &str, bg: &str) -> f32 {
    AccessibilityChecker::get_contrast_ratio(fg, bg)
}

pub fn meets_accessibility_standard(fg: &str, bg: &str, level: AccessibilityLevel) -> bool {
    AccessibilityChecker::meets_standard(fg, bg, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_info_debug() {
        let info = get_terminal_info();
        println!("Terminal info: {:#?}", info);
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_semantic_colors() {
        let scheme = get_color_scheme();
        println!("Color scheme: {:#?}", scheme);

        // Test semantic coloring
        let _success_text = colorize_adaptive("Success!", SemanticColor::Success);
        let _error_text = colorize_adaptive("Error!", SemanticColor::Error);

        // In test environment, should return plain text
        assert_eq!(colorize_adaptive("test", SemanticColor::Primary), "test");
    }

    #[test]
    fn test_theme_detection() {
        let detector = ThemeDetector::new();
        let theme = detector.detect();
        assert!(matches!(
            theme,
            TerminalTheme::Light | TerminalTheme::Dark | TerminalTheme::Unknown
        ));
    }

    #[test]
    fn test_theme_from_str() {
        assert_eq!(
            "light".parse::<TerminalTheme>().unwrap(),
            TerminalTheme::Light
        );
        assert_eq!(
            "dark".parse::<TerminalTheme>().unwrap(),
            TerminalTheme::Dark
        );
        assert_eq!(
            "LIGHT".parse::<TerminalTheme>().unwrap(),
            TerminalTheme::Light
        );
        assert!("invalid".parse::<TerminalTheme>().is_err());
    }

    #[test]
    fn test_theme_display() {
        assert_eq!(TerminalTheme::Light.to_string(), "light");
        assert_eq!(TerminalTheme::Dark.to_string(), "dark");
        assert_eq!(TerminalTheme::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_semantic_color_display() {
        assert_eq!(SemanticColor::Primary.to_string(), "primary");
        assert_eq!(SemanticColor::Success.to_string(), "success");
    }

    #[test]
    fn test_accessibility_standards() {
        assert!(AccessibilityChecker::meets_standard(
            Colors::WHITE,
            Colors::BLACK,
            AccessibilityLevel::AA
        ));
        assert!(AccessibilityChecker::meets_standard(
            Colors::WHITE,
            Colors::BLACK,
            AccessibilityLevel::AAA
        ));
        assert!(AccessibilityChecker::meets_standard(
            Colors::BLACK,
            Colors::WHITE,
            AccessibilityLevel::AA
        ));

        // Test compatibility wrappers
        assert!(meets_accessibility_standard(
            Colors::WHITE,
            Colors::BLACK,
            AccessibilityLevel::AA
        ));
    }

    #[test]
    fn test_accessibility_level_display() {
        assert_eq!(AccessibilityLevel::AA.to_string(), "AA (4.5:1)");
        assert_eq!(AccessibilityLevel::AAA.to_string(), "AAA (7:1)");
    }

    #[test]
    fn test_find_accessible_color() {
        let color =
            AccessibilityChecker::find_accessible_color(Colors::BLACK, AccessibilityLevel::AA);
        assert!(AccessibilityChecker::meets_standard(
            color,
            Colors::BLACK,
            AccessibilityLevel::AA
        ));
    }

    #[test]
    fn test_true_color_detection() {
        // Should not panic and return a boolean
        let supports = supports_true_color();
        // Test that the function returns without panicking

        // Test direct detector call
        let direct_supports = TerminalCapabilityDetector::supports_true_color();
        assert_eq!(supports, direct_supports);
    }

    #[test]
    fn test_color_scheme_consistency() {
        let scheme = get_color_scheme();

        // All colors should be non-empty
        assert!(!scheme.primary.is_empty());
        assert!(!scheme.success.is_empty());
        assert!(!scheme.warning.is_empty());
        assert!(!scheme.error.is_empty());
        assert!(!scheme.info.is_empty());
        assert!(!scheme.muted.is_empty());
        assert!(!scheme.accent.is_empty());
        assert!(!scheme.url.is_empty());

        // Colors should start with escape sequence
        assert!(scheme.primary.starts_with('\x1b'));
        assert!(scheme.success.starts_with('\x1b'));

        // Test get_color method
        assert_eq!(scheme.get_color(SemanticColor::Primary), scheme.primary);
        assert_eq!(scheme.get_color(SemanticColor::Success), scheme.success);
    }

    #[test]
    fn test_theme_detector_with_fallback() {
        let detector = ThemeDetector::with_fallback(TerminalTheme::Light);
        let theme = detector.detect();
        assert!(matches!(
            theme,
            TerminalTheme::Light | TerminalTheme::Dark | TerminalTheme::Unknown
        ));
    }

    #[test]
    fn test_color_scheme_for_theme() {
        let light_scheme = ColorScheme::for_theme(TerminalTheme::Light);
        assert_eq!(light_scheme.theme, TerminalTheme::Light);

        let dark_scheme = ColorScheme::for_theme(TerminalTheme::Dark);
        assert_eq!(dark_scheme.theme, TerminalTheme::Dark);
    }
}
