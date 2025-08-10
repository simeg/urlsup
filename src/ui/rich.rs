//! Rich text formatting for terminal output
//!
//! Provides markdown-style formatting, table layouts, and enhanced
//! visual elements for better user experience in terminal environments.

use crate::ui::color::{Colors, colorize};
use crate::ui::theme::{SemanticColor, colorize_adaptive};
use std::fmt;

/// Rich text formatting elements
#[derive(Debug, Clone)]
pub struct RichText {
    elements: Vec<RichElement>,
}

#[derive(Debug, Clone)]
pub enum RichElement {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, url: String },
    Heading { level: u8, text: String },
    List { items: Vec<String>, ordered: bool },
    Table(Table),
    Separator,
    Newline,
}

/// Error types for rich text rendering
#[derive(Debug)]
pub enum RichTextError {
    /// Invalid table structure
    InvalidTable(String),
    /// Text width calculation error
    WidthCalculation(String),
    /// Rendering failure
    RenderError(String),
}

impl fmt::Display for RichTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTable(msg) => write!(f, "Invalid table: {}", msg),
            Self::WidthCalculation(msg) => write!(f, "Width calculation error: {}", msg),
            Self::RenderError(msg) => write!(f, "Render error: {}", msg),
        }
    }
}

impl std::error::Error for RichTextError {}

/// Table structure for rich formatting
#[derive(Debug, Clone)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub alignment: Vec<TableAlignment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlignment {
    Left,
    Center,
    Right,
}

impl Default for TableAlignment {
    fn default() -> Self {
        Self::Left
    }
}

impl fmt::Display for TableAlignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Center => write!(f, "center"),
            Self::Right => write!(f, "right"),
        }
    }
}

impl RichText {
    /// Create a new rich text builder
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
        }
    }

    /// Add plain text
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.elements.push(RichElement::Text(text.into()));
        self
    }

    /// Add bold text
    pub fn bold(mut self, text: impl Into<String>) -> Self {
        self.elements.push(RichElement::Bold(text.into()));
        self
    }

    /// Add italic text (using dim for terminal compatibility)
    pub fn italic(mut self, text: impl Into<String>) -> Self {
        self.elements.push(RichElement::Italic(text.into()));
        self
    }

    /// Add inline code
    pub fn code(mut self, text: impl Into<String>) -> Self {
        self.elements.push(RichElement::Code(text.into()));
        self
    }

    /// Add a link
    pub fn link(mut self, text: impl Into<String>, url: impl Into<String>) -> Self {
        self.elements.push(RichElement::Link {
            text: text.into(),
            url: url.into(),
        });
        self
    }

    /// Add a heading
    pub fn heading(mut self, level: u8, text: impl Into<String>) -> Self {
        self.elements.push(RichElement::Heading {
            level: level.clamp(1, 6),
            text: text.into(),
        });
        self
    }

    /// Add an unordered list
    pub fn list(mut self, items: Vec<String>) -> Self {
        self.elements.push(RichElement::List {
            items,
            ordered: false,
        });
        self
    }

    /// Add an ordered list
    pub fn ordered_list(mut self, items: Vec<String>) -> Self {
        self.elements.push(RichElement::List {
            items,
            ordered: true,
        });
        self
    }

    /// Add a table
    pub fn table(mut self, table: Table) -> Self {
        self.elements.push(RichElement::Table(table));
        self
    }

    /// Add a separator line
    pub fn separator(mut self) -> Self {
        self.elements.push(RichElement::Separator);
        self
    }

    /// Add a newline
    pub fn newline(mut self) -> Self {
        self.elements.push(RichElement::Newline);
        self
    }

    /// Render the rich text to a string
    pub fn render(&self) -> String {
        let mut output = String::new();

        for element in &self.elements {
            match element {
                RichElement::Text(text) => {
                    output.push_str(text);
                }
                RichElement::Bold(text) => {
                    output.push_str(&format!("{}{}{}", Colors::BOLD, text, Colors::RESET));
                }
                RichElement::Italic(text) => {
                    output.push_str(&colorize(text, Colors::DIM));
                }
                RichElement::Code(text) => {
                    output.push_str(&format!(
                        "{}{}{}{}{}",
                        colorize("`", Colors::DIM),
                        Colors::BOLD,
                        text,
                        Colors::RESET,
                        colorize("`", Colors::DIM)
                    ));
                }
                RichElement::Link { text, url } => {
                    output.push_str(&format!(
                        "{}{}{}",
                        colorize_adaptive(text, SemanticColor::Url),
                        colorize(" (", Colors::DIM),
                        colorize(&format!("{})", url), Colors::DIM)
                    ));
                }
                RichElement::Heading { level, text } => {
                    let prefix = "#".repeat(*level as usize);
                    output.push_str(&format!(
                        "{} {}\n",
                        colorize(&prefix, Colors::BRIGHT_BLUE),
                        colorize_adaptive(text, SemanticColor::Primary)
                    ));
                }
                RichElement::List { items, ordered } => {
                    for (i, item) in items.iter().enumerate() {
                        let marker = if *ordered {
                            format!("{}.", i + 1)
                        } else {
                            "•".to_string()
                        };
                        output.push_str(&format!(
                            "  {} {}\n",
                            colorize(&marker, Colors::BRIGHT_BLUE),
                            item
                        ));
                    }
                }
                RichElement::Table(table) => {
                    output.push_str(&render_table(table));
                }
                RichElement::Separator => {
                    let width = get_terminal_width().unwrap_or(80);
                    let separator = "─".repeat(width.min(80));
                    output.push_str(&format!("{}\n", colorize(&separator, Colors::DIM)));
                }
                RichElement::Newline => {
                    output.push('\n');
                }
            }
        }

        output
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self::new()
    }
}

/// Table builder for easier table creation
pub struct TableBuilder {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    alignment: Vec<TableAlignment>,
}

impl TableBuilder {
    /// Create a new table builder
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            alignment: Vec::new(),
        }
    }

    /// Set table headers
    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.alignment = vec![TableAlignment::Left; headers.len()];
        self.headers = headers;
        self
    }

    /// Set column alignment
    pub fn alignment(mut self, alignment: Vec<TableAlignment>) -> Self {
        self.alignment = alignment;
        self
    }

    /// Add a row
    pub fn row(mut self, row: Vec<String>) -> Self {
        self.rows.push(row);
        self
    }

    /// Build the table
    pub fn build(self) -> Table {
        Table {
            headers: self.headers,
            rows: self.rows,
            alignment: self.alignment,
        }
    }
}

impl Default for TableBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a table with proper formatting
fn render_table(table: &Table) -> String {
    if table.headers.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let num_cols = table.headers.len();

    // Calculate column widths
    let mut col_widths = vec![0; num_cols];

    // Check header widths
    for (i, header) in table.headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(header.len());
    }

    // Check row widths
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate().take(num_cols) {
            col_widths[i] = col_widths[i].max(cell.len());
        }
    }

    // Render header
    output.push('┌');
    for (i, width) in col_widths.iter().enumerate() {
        output.push_str(&"─".repeat(width + 2));
        if i < col_widths.len() - 1 {
            output.push('┬');
        }
    }
    output.push_str("┐\n");

    // Header content
    output.push('│');
    for (i, header) in table.headers.iter().enumerate() {
        output.push(' ');
        output.push_str(&colorize_adaptive(header, SemanticColor::Primary));
        output.push_str(&" ".repeat(col_widths[i] - header.len()));
        output.push_str(" │");
    }
    output.push('\n');

    // Header separator
    output.push('├');
    for (i, width) in col_widths.iter().enumerate() {
        output.push_str(&"─".repeat(width + 2));
        if i < col_widths.len() - 1 {
            output.push('┼');
        }
    }
    output.push_str("┤\n");

    // Render rows
    for row in &table.rows {
        output.push('│');
        for (i, cell) in row.iter().enumerate().take(num_cols) {
            output.push(' ');

            let aligned_cell = match table.alignment.get(i).unwrap_or(&TableAlignment::Left) {
                TableAlignment::Left => {
                    format!("{}{}", cell, " ".repeat(col_widths[i] - cell.len()))
                }
                TableAlignment::Right => {
                    format!("{}{}", " ".repeat(col_widths[i] - cell.len()), cell)
                }
                TableAlignment::Center => {
                    let padding = col_widths[i] - cell.len();
                    let left_pad = padding / 2;
                    let right_pad = padding - left_pad;
                    format!("{}{}{}", " ".repeat(left_pad), cell, " ".repeat(right_pad))
                }
            };

            output.push_str(&aligned_cell);
            output.push_str(" │");
        }
        output.push('\n');
    }

    // Bottom border
    output.push('└');
    for (i, width) in col_widths.iter().enumerate() {
        output.push_str(&"─".repeat(width + 2));
        if i < col_widths.len() - 1 {
            output.push('┴');
        }
    }
    output.push_str("┘\n");

    colorize(&output, Colors::DIM)
}

/// Get terminal width for formatting
fn get_terminal_width() -> Option<usize> {
    // Try to get terminal size
    if let Some((w, _)) = term_size::dimensions() {
        Some(w)
    } else {
        None
    }
}

/// Progress bar with rich formatting
pub struct ProgressBar {
    current: usize,
    total: usize,
    width: usize,
    message: String,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new(total: usize) -> Self {
        Self {
            current: 0,
            total,
            width: 40,
            message: String::new(),
        }
    }

    /// Set progress bar width
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Set progress message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Update progress
    pub fn set_progress(&mut self, current: usize) {
        self.current = current.min(self.total);
    }

    /// Render progress bar
    pub fn render(&self) -> String {
        if self.total == 0 {
            return String::new();
        }

        let progress = self.current as f64 / self.total as f64;
        let filled = (progress * self.width as f64) as usize;
        let empty = self.width - filled;

        let bar = format!(
            "{}{}{}{}{}",
            colorize("[", Colors::DIM),
            colorize_adaptive(&"█".repeat(filled), SemanticColor::Success),
            colorize(&"░".repeat(empty), Colors::DIM),
            colorize("]", Colors::DIM),
            if !self.message.is_empty() {
                format!(" {}", self.message)
            } else {
                String::new()
            }
        );

        let percentage = (progress * 100.0) as usize;
        format!(
            "{} {}",
            bar,
            colorize_adaptive(&format!("{}%", percentage), SemanticColor::Info)
        )
    }
}

/// Create a formatted info box
pub fn info_box(title: &str, content: &str) -> String {
    let width = get_terminal_width().unwrap_or(80).min(80);
    let content_width = width - 4; // Account for borders and padding

    let mut lines = Vec::new();
    for line in content.lines() {
        if line.len() <= content_width {
            lines.push(line.to_string());
        } else {
            // Simple word wrapping
            let words: Vec<&str> = line.split_whitespace().collect();
            let mut current_line = String::new();

            for word in words {
                if current_line.len() + word.len() < content_width {
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = word.to_string();
                } else {
                    lines.push(word.to_string());
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }

    let mut output = String::new();

    // Top border
    output.push_str(&colorize("╭", Colors::BRIGHT_BLUE));
    output.push_str(&colorize(&"─".repeat(width - 2), Colors::BRIGHT_BLUE));
    output.push_str(&colorize("╮\n", Colors::BRIGHT_BLUE));

    // Title
    if !title.is_empty() {
        let title_padding = (content_width - title.len()) / 2;
        output.push_str(&colorize("│ ", Colors::BRIGHT_BLUE));
        output.push_str(&" ".repeat(title_padding));
        output.push_str(&colorize_adaptive(title, SemanticColor::Primary));
        output.push_str(&" ".repeat(content_width - title_padding - title.len()));
        output.push_str(&colorize(" │\n", Colors::BRIGHT_BLUE));

        // Separator
        output.push_str(&colorize("├", Colors::BRIGHT_BLUE));
        output.push_str(&colorize(&"─".repeat(width - 2), Colors::BRIGHT_BLUE));
        output.push_str(&colorize("┤\n", Colors::BRIGHT_BLUE));
    }

    // Content
    for line in lines {
        output.push_str(&colorize("│ ", Colors::BRIGHT_BLUE));
        output.push_str(&line);
        output.push_str(&" ".repeat(content_width - line.len()));
        output.push_str(&colorize(" │\n", Colors::BRIGHT_BLUE));
    }

    // Bottom border
    output.push_str(&colorize("╰", Colors::BRIGHT_BLUE));
    output.push_str(&colorize(&"─".repeat(width - 2), Colors::BRIGHT_BLUE));
    output.push_str(&colorize("╯\n", Colors::BRIGHT_BLUE));

    output
}

/// Terminal width detector with caching
pub struct TerminalWidthDetector {
    cached_width: Option<usize>,
}

impl Default for TerminalWidthDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalWidthDetector {
    pub fn new() -> Self {
        Self { cached_width: None }
    }

    pub fn get_width(&mut self) -> Option<usize> {
        if self.cached_width.is_none() {
            self.cached_width = term_size::dimensions().map(|(w, _)| w);
        }
        self.cached_width
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_width = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rich_text_builder() {
        let rich = RichText::new()
            .text("Regular text ")
            .bold("bold text ")
            .italic("italic text ")
            .code("code")
            .newline()
            .link("GitHub", "https://github.com");

        let output = rich.render();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_table_builder() {
        let table = TableBuilder::new()
            .headers(vec!["Name".to_string(), "Age".to_string()])
            .row(vec!["Alice".to_string(), "30".to_string()])
            .row(vec!["Bob".to_string(), "25".to_string()])
            .build();

        assert_eq!(table.headers.len(), 2);
        assert_eq!(table.rows.len(), 2);
    }

    #[test]
    fn test_progress_bar() {
        let mut bar = ProgressBar::new(100)
            .width(20)
            .message("Processing...".to_string());

        bar.set_progress(50);
        let output = bar.render();
        assert!(!output.is_empty());
        assert!(output.contains("50%"));
    }

    #[test]
    fn test_info_box() {
        let box_content = info_box("Test", "This is a test message");
        assert!(!box_content.is_empty());
        assert!(box_content.contains("Test"));
        assert!(box_content.contains("This is a test message"));
    }

    #[test]
    fn test_table_rendering() {
        let table = Table {
            headers: vec!["Col1".to_string(), "Col2".to_string()],
            rows: vec![
                vec!["A".to_string(), "B".to_string()],
                vec!["C".to_string(), "D".to_string()],
            ],
            alignment: vec![TableAlignment::Left, TableAlignment::Right],
        };

        let output = render_table(&table);
        assert!(!output.is_empty());
        assert!(output.contains("Col1"));
        assert!(output.contains("Col2"));
    }

    #[test]
    fn test_table_alignment() {
        let alignments = [
            TableAlignment::Left,
            TableAlignment::Center,
            TableAlignment::Right,
        ];
        for alignment in alignments {
            let table = Table {
                headers: vec!["Test".to_string()],
                rows: vec![vec!["Content".to_string()]],
                alignment: vec![alignment],
            };
            let output = render_table(&table);
            assert!(!output.is_empty());

            // Test alignment display
            assert!(!format!("{}", alignment).is_empty());
        }
    }

    #[test]
    fn test_table_alignment_default() {
        assert_eq!(TableAlignment::default(), TableAlignment::Left);
    }

    #[test]
    fn test_rich_text_error_types() {
        let error = RichTextError::InvalidTable("test error".to_string());
        assert!(format!("{}", error).contains("test error"));

        let width_error = RichTextError::WidthCalculation("width issue".to_string());
        assert!(format!("{}", width_error).contains("width issue"));
    }

    #[test]
    fn test_terminal_width_detector() {
        let mut detector = TerminalWidthDetector::new();

        // First call should calculate width
        let width1 = detector.get_width();

        // Second call should use cached value
        let width2 = detector.get_width();
        assert_eq!(width1, width2);

        // Invalidate cache and get fresh value
        detector.invalidate_cache();
        let width3 = detector.get_width();
        // Should be same as before, but freshly calculated
        assert_eq!(width1, width3);
    }
}
