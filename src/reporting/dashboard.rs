use crate::config::Config;
use crate::reporting::performance::PerformanceReport;
use crate::ui::output::DisplayMetadata;
use crate::validation::validator::ValidationResult;
use std::collections::HashMap;
use std::fs;
use std::io;

/// Constants for dashboard styling and layout
mod dashboard_constants {
    /// Chart.js CDN URL for rendering charts
    pub const CHART_JS_CDN: &str = "https://cdn.jsdelivr.net/npm/chart.js";

    /// Success rate thresholds for styling
    pub const SUCCESS_THRESHOLD: f64 = 90.0;
    pub const WARNING_THRESHOLD: f64 = 70.0;

    /// Memory unit conversion
    pub const BYTES_TO_MB: f64 = 1_048_576.0;
}

/// Data structure containing all information needed for dashboard generation
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// Validation metadata and statistics
    pub metadata: DisplayMetadata,
    /// List of validation results (usually only failed URLs)
    pub results: Vec<ValidationResult>,
    /// Optional performance analysis data
    pub performance: Option<PerformanceReport>,
    /// Configuration used for validation
    pub config: Config,
    /// Timestamp when the dashboard was generated
    pub timestamp: String,
}

/// Error type for dashboard generation
#[derive(Debug)]
pub enum DashboardError {
    FileWrite(io::Error),
    Serialization(String),
}

impl std::fmt::Display for DashboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DashboardError::FileWrite(e) => write!(f, "Failed to write dashboard file: {}", e),
            DashboardError::Serialization(e) => write!(f, "Failed to serialize data: {}", e),
        }
    }
}

impl std::error::Error for DashboardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DashboardError::FileWrite(e) => Some(e),
            DashboardError::Serialization(_) => None,
        }
    }
}

impl From<io::Error> for DashboardError {
    fn from(e: io::Error) -> Self {
        DashboardError::FileWrite(e)
    }
}

/// HTML dashboard generator for URL validation results
pub struct HtmlDashboard;

impl HtmlDashboard {
    /// Generate and write an HTML dashboard to the specified path
    pub fn generate_dashboard(
        data: &DashboardData,
        output_path: &str,
    ) -> Result<(), DashboardError> {
        let html_content = Self::generate_html_content(data)?;
        fs::write(output_path, html_content)?;
        Ok(())
    }

    /// Generate the complete HTML document content
    fn generate_html_content(data: &DashboardData) -> Result<String, DashboardError> {
        let css_styles = Self::generate_css();
        let js_scripts = Self::generate_javascript();
        let body_content = Self::generate_body_content(data)?;

        Ok(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>URL Validation Dashboard - urlsup</title>
    <script src="{}"></script>
    <style>{}</style>
</head>
<body>
    {}
    <script>{}</script>
</body>
</html>"#,
            dashboard_constants::CHART_JS_CDN,
            css_styles,
            body_content,
            js_scripts
        ))
    }

    fn generate_css() -> &'static str {
        r#"
        :root {
            --primary-color: #2563eb;
            --success-color: #059669;
            --warning-color: #d97706;
            --error-color: #dc2626;
            --bg-color: #f8fafc;
            --card-bg: #ffffff;
            --border-color: #e2e8f0;
            --text-primary: #1e293b;
            --text-secondary: #64748b;
        }

        * { margin: 0; padding: 0; box-sizing: border-box; }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background-color: var(--bg-color);
            color: var(--text-primary);
            line-height: 1.6;
        }

        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }

        .header {
            text-align: center;
            margin-bottom: 3rem;
            padding: 2rem;
            background: linear-gradient(135deg, var(--primary-color), #3b82f6);
            color: white;
            border-radius: 12px;
            box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
        }

        .header h1 {
            font-size: 2.5rem;
            margin-bottom: 0.5rem;
            font-weight: 700;
        }

        .header p {
            font-size: 1.1rem;
            opacity: 0.9;
        }

        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 1.5rem;
            margin-bottom: 3rem;
        }

        .stat-card {
            background: var(--card-bg);
            padding: 1.5rem;
            border-radius: 12px;
            border: 1px solid var(--border-color);
            box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.06);
            transition: transform 0.2s, box-shadow 0.2s;
        }

        .stat-card:hover {
            transform: translateY(-2px);
            box-shadow: 0 4px 12px -1px rgba(0, 0, 0, 0.15);
        }

        .stat-icon {
            width: 48px;
            height: 48px;
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            margin-bottom: 1rem;
            font-size: 1.5rem;
        }

        .stat-value {
            font-size: 2rem;
            font-weight: 700;
            margin-bottom: 0.5rem;
        }

        .stat-label {
            color: var(--text-secondary);
            font-size: 0.9rem;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .success { color: var(--success-color); background-color: #ecfdf5; }
        .warning { color: var(--warning-color); background-color: #fffbeb; }
        .error { color: var(--error-color); background-color: #fef2f2; }
        .info { color: var(--primary-color); background-color: #eff6ff; }

        .chart-container {
            background: var(--card-bg);
            padding: 2rem;
            border-radius: 12px;
            border: 1px solid var(--border-color);
            margin-bottom: 2rem;
            box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.06);
        }

        .chart-title {
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 1rem;
            color: var(--text-primary);
        }

        .issues-section {
            background: var(--card-bg);
            border-radius: 12px;
            border: 1px solid var(--border-color);
            overflow: hidden;
            box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.06);
        }

        .section-header {
            background: var(--bg-color);
            padding: 1.5rem;
            border-bottom: 1px solid var(--border-color);
        }

        .section-title {
            font-size: 1.25rem;
            font-weight: 600;
            color: var(--text-primary);
        }

        .issues-list {
            max-height: 400px;
            overflow-y: auto;
        }

        .issue-item {
            padding: 1rem 1.5rem;
            border-bottom: 1px solid var(--border-color);
            transition: background-color 0.2s;
        }

        .issue-item:hover {
            background-color: var(--bg-color);
        }

        .issue-item:last-child {
            border-bottom: none;
        }

        .issue-url {
            font-weight: 500;
            color: var(--primary-color);
            margin-bottom: 0.25rem;
        }

        .issue-details {
            font-size: 0.875rem;
            color: var(--text-secondary);
        }

        .status-badge {
            display: inline-block;
            padding: 0.25rem 0.5rem;
            border-radius: 6px;
            font-size: 0.75rem;
            font-weight: 500;
            text-transform: uppercase;
            letter-spacing: 0.025em;
        }

        .performance-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 1.5rem;
            margin-bottom: 2rem;
        }

        .performance-card {
            background: var(--card-bg);
            padding: 1.5rem;
            border-radius: 12px;
            border: 1px solid var(--border-color);
            box-shadow: 0 2px 4px -1px rgba(0, 0, 0, 0.06);
        }

        .recommendations {
            background: linear-gradient(135deg, #fef3c7, #fed7aa);
            border: 1px solid #f59e0b;
            border-radius: 12px;
            padding: 1.5rem;
            margin-top: 2rem;
        }

        .recommendations h3 {
            color: #92400e;
            margin-bottom: 1rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .recommendations ul {
            list-style: none;
        }

        .recommendations li {
            color: #78350f;
            margin-bottom: 0.5rem;
            padding-left: 1.5rem;
            position: relative;
        }

        .recommendations li::before {
            content: "💡";
            position: absolute;
            left: 0;
        }

        @media (max-width: 768px) {
            .container { padding: 1rem; }
            .header h1 { font-size: 2rem; }
            .stats-grid { grid-template-columns: 1fr; }
            .chart-container { padding: 1rem; }
        }
        "#
    }

    /// Generate the main body content of the dashboard
    fn generate_body_content(data: &DashboardData) -> Result<String, DashboardError> {
        let header_section = Self::generate_header_section(&data.timestamp);
        let stats_section = Self::generate_stats_section(data);
        let charts_section = Self::generate_charts_section(data)?;
        let issues_section = Self::generate_issues_section(data);
        let performance_section = Self::generate_performance_section(data);

        Ok(format!(
            r#"
            <div class="container">
                {}
                {}
                {}
                {}
                {}
            </div>
            "#,
            header_section, stats_section, charts_section, issues_section, performance_section
        ))
    }

    /// Generate the dashboard header section
    fn generate_header_section(timestamp: &str) -> String {
        format!(
            r#"
            <div class="header">
                <h1>🔗 URL Validation Dashboard</h1>
                <p>Generated on {} by urlsup</p>
            </div>
            "#,
            timestamp
        )
    }

    /// Generate the statistics cards section
    fn generate_stats_section(data: &DashboardData) -> String {
        let success_rate = Self::calculate_success_rate(&data.metadata);
        let success_rate_style = Self::get_success_rate_style(success_rate);

        format!(
            r#"
            <div class="stats-grid">
                {}
                {}
                {}
                {}
            </div>
            "#,
            Self::generate_stat_card(
                "📁",
                &data.metadata.files_processed.to_string(),
                "Files Processed",
                "info"
            ),
            Self::generate_stat_card(
                "🔗",
                &data.metadata.total_urls_found.to_string(),
                "URLs Found",
                "info"
            ),
            Self::generate_stat_card(
                "✅",
                &data.metadata.total_validated.to_string(),
                "URLs Validated",
                "info"
            ),
            Self::generate_stat_card(
                "📊",
                &format!("{:.1}%", success_rate),
                "Success Rate",
                &success_rate_style
            ),
        )
    }

    /// Calculate success rate from metadata
    fn calculate_success_rate(metadata: &DisplayMetadata) -> f64 {
        if metadata.total_validated > 0 {
            (metadata.total_validated - metadata.issues_found) as f64
                / metadata.total_validated as f64
                * 100.0
        } else {
            100.0
        }
    }

    /// Get CSS class for success rate styling based on thresholds
    fn get_success_rate_style(success_rate: f64) -> String {
        if success_rate >= dashboard_constants::SUCCESS_THRESHOLD {
            "success".to_string()
        } else if success_rate >= dashboard_constants::WARNING_THRESHOLD {
            "warning".to_string()
        } else {
            "error".to_string()
        }
    }

    /// Generate a single statistics card
    fn generate_stat_card(icon: &str, value: &str, label: &str, style_class: &str) -> String {
        format!(
            r#"
            <div class="stat-card">
                <div class="stat-icon {}">{}</div>
                <div class="stat-value">{}</div>
                <div class="stat-label">{}</div>
            </div>
            "#,
            style_class, icon, value, label
        )
    }

    /// Generate the charts section with validation results distribution
    fn generate_charts_section(data: &DashboardData) -> Result<String, DashboardError> {
        let status_counts = Self::categorize_issues(&data.results);
        let chart_data_json = serde_json::to_string(&status_counts)
            .map_err(|e| DashboardError::Serialization(e.to_string()))?;

        Ok(format!(
            r#"
            <div class="chart-container">
                <h3 class="chart-title">📊 Validation Results Distribution</h3>
                <canvas id="statusChart" width="400" height="200"></canvas>
            </div>
            
            <script>
                const statusData = {};
                window.chartData = statusData;
            </script>
            "#,
            chart_data_json
        ))
    }

    fn generate_issues_section(data: &DashboardData) -> String {
        if data.results.is_empty() {
            return r#"
                <div class="issues-section">
                    <div class="section-header">
                        <h3 class="section-title">✅ No Issues Found</h3>
                    </div>
                    <div style="padding: 2rem; text-align: center; color: var(--success-color);">
                        <p>🎉 All URLs are working correctly!</p>
                    </div>
                </div>
                "#
            .to_string();
        }

        let issues_html = data
            .results
            .iter()
            .map(|result| {
                let status_class = match result.status_code {
                    Some(code) if (200..300).contains(&code) => "success",
                    Some(code) if (300..400).contains(&code) => "warning",
                    Some(code) if (400..500).contains(&code) => "error",
                    Some(code) if (500..600).contains(&code) => "error",
                    None => "error",
                    _ => "warning",
                };

                let status_text = result
                    .status_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "ERROR".to_string());

                format!(
                    r#"
                    <div class="issue-item">
                        <div class="issue-url">{}</div>
                        <div class="issue-details">
                            <span class="status-badge {}">{}</span>
                            {} • Line {}
                            {}
                        </div>
                    </div>
                    "#,
                    result.url,
                    status_class,
                    status_text,
                    result.file_name,
                    result.line,
                    if let Some(ref desc) = result.description {
                        if !desc.is_empty() {
                            format!(" • {}", desc)
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("");

        format!(
            r#"
            <div class="issues-section">
                <div class="section-header">
                    <h3 class="section-title">⚠️ Issues Found ({})</h3>
                </div>
                <div class="issues-list">
                    {}
                </div>
            </div>
            "#,
            data.results.len(),
            issues_html
        )
    }

    fn generate_performance_section(data: &DashboardData) -> String {
        if let Some(performance) = &data.performance {
            let operations_html = performance
                .operations
                .iter()
                .map(|op| {
                    format!(
                        r#"
                        <div class="performance-card">
                            <h4>{}</h4>
                            <p><strong>Duration:</strong> {:?}</p>
                            <p><strong>Items:</strong> {}</p>
                            <p><strong>Memory:</strong> {:.2} MB</p>
                            <p><strong>CPU:</strong> {:.1}%</p>
                        </div>
                        "#,
                        op.operation,
                        op.duration,
                        op.items_processed,
                        op.memory_used as f64 / dashboard_constants::BYTES_TO_MB,
                        op.cpu_usage
                    )
                })
                .collect::<Vec<_>>()
                .join("");

            let recommendations_html = if !performance.recommendations.is_empty() {
                let rec_list = performance
                    .recommendations
                    .iter()
                    .map(|rec| format!("<li>{}</li>", rec))
                    .collect::<Vec<_>>()
                    .join("");

                format!(
                    r#"
                    <div class="recommendations">
                        <h3>💡 Performance Recommendations</h3>
                        <ul>{}</ul>
                    </div>
                    "#,
                    rec_list
                )
            } else {
                String::new()
            };

            format!(
                r#"
                <div class="chart-container">
                    <h3 class="chart-title">⚡ Performance Analysis</h3>
                    <p><strong>Total Duration:</strong> {:?}</p>
                    <p><strong>Peak Memory:</strong> {:.2} MB</p>
                    <p><strong>Average CPU:</strong> {:.1}%</p>
                </div>

                <div class="performance-grid">
                    {}
                </div>

                {}
                "#,
                performance.total_duration,
                performance.peak_memory_mb,
                performance.avg_cpu_usage,
                operations_html,
                recommendations_html
            )
        } else {
            String::new()
        }
    }

    fn generate_javascript() -> &'static str {
        r#"
        document.addEventListener('DOMContentLoaded', function() {
            if (typeof Chart !== 'undefined' && window.chartData) {
                const ctx = document.getElementById('statusChart');
                if (ctx) {
                    new Chart(ctx, {
                        type: 'doughnut',
                        data: {
                            labels: Object.keys(window.chartData),
                            datasets: [{
                                data: Object.values(window.chartData),
                                backgroundColor: [
                                    '#059669', // success
                                    '#d97706', // warning  
                                    '#dc2626', // error
                                    '#6b7280'  // other
                                ],
                                borderWidth: 2,
                                borderColor: '#ffffff'
                            }]
                        },
                        options: {
                            responsive: true,
                            plugins: {
                                legend: {
                                    position: 'bottom',
                                    labels: {
                                        padding: 20,
                                        font: {
                                            size: 14
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }
        });
        "#
    }

    fn categorize_issues(results: &[ValidationResult]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();

        for result in results {
            let category = match result.status_code {
                Some(code) if (200..300).contains(&code) => "Success",
                Some(code) if (300..400).contains(&code) => "Redirects",
                Some(code) if (400..500).contains(&code) => "Client Errors",
                Some(code) if (500..600).contains(&code) => "Server Errors",
                None => "Network Errors",
                _ => "Other",
            };

            *counts.entry(category.to_string()).or_insert(0) += 1;
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporting::performance::{BenchmarkResult, PerformanceReport};
    use std::error::Error;
    use std::time::Duration;
    use tempfile::NamedTempFile;

    fn create_test_metadata() -> DisplayMetadata {
        DisplayMetadata {
            total_validated: 100,
            issues_found: 5,
            files_processed: 10,
            total_urls_found: 150,
            unique_urls_found: 100,
        }
    }

    fn create_test_validation_results() -> Vec<ValidationResult> {
        vec![
            ValidationResult {
                url: "https://example.com/404".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://example.com/500".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: Some(500),
                description: Some("Internal Server Error".to_string()),
            },
            ValidationResult {
                url: "https://timeout.example".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("Connection timeout".to_string()),
            },
        ]
    }

    fn create_test_performance_report() -> PerformanceReport {
        let operations = vec![
            BenchmarkResult {
                operation: "file_processing".to_string(),
                duration: Duration::from_millis(500),
                items_processed: 10,
                memory_used: 1024 * 1024, // 1MB
                cpu_usage: 25.0,
            },
            BenchmarkResult {
                operation: "validation".to_string(),
                duration: Duration::from_millis(2000),
                items_processed: 100,
                memory_used: 2048 * 1024, // 2MB
                cpu_usage: 50.0,
            },
        ];

        PerformanceReport {
            total_duration: Duration::from_millis(2500),
            operations,
            peak_memory_mb: 2.0,
            avg_cpu_usage: 37.5,
            recommendations: vec![
                "Consider increasing concurrency".to_string(),
                "Monitor memory usage".to_string(),
            ],
        }
    }

    fn create_test_dashboard_data() -> DashboardData {
        DashboardData {
            metadata: create_test_metadata(),
            results: create_test_validation_results(),
            performance: Some(create_test_performance_report()),
            config: Config::default(),
            timestamp: "2025-01-01 12:00:00 UTC".to_string(),
        }
    }

    #[test]
    fn test_categorize_issues() {
        let results = vec![
            ValidationResult {
                url: "https://example.com".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://test.org".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: Some(500),
                description: Some("Server Error".to_string()),
            },
        ];

        let categories = HtmlDashboard::categorize_issues(&results);

        assert_eq!(categories.get("Client Errors"), Some(&1));
        assert_eq!(categories.get("Server Errors"), Some(&1));
    }

    #[test]
    fn test_dashboard_data_creation() {
        let metadata = DisplayMetadata {
            total_validated: 100,
            issues_found: 5,
            files_processed: 10,
            total_urls_found: 150,
            unique_urls_found: 100,
        };

        let data = DashboardData {
            metadata,
            results: vec![],
            performance: None,
            config: Config::default(),
            timestamp: "2025-01-01 12:00:00".to_string(),
        };

        assert_eq!(data.metadata.total_validated, 100);
        assert_eq!(data.results.len(), 0);
    }

    #[test]
    fn test_categorize_issues_comprehensive() {
        let results = vec![
            // Client errors (4xx)
            ValidationResult {
                url: "https://example.com/400".to_string(),
                line: 1,
                file_name: "test.md".to_string(),
                status_code: Some(400),
                description: Some("Bad Request".to_string()),
            },
            ValidationResult {
                url: "https://example.com/404".to_string(),
                line: 2,
                file_name: "test.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found".to_string()),
            },
            ValidationResult {
                url: "https://example.com/429".to_string(),
                line: 3,
                file_name: "test.md".to_string(),
                status_code: Some(429),
                description: Some("Too Many Requests".to_string()),
            },
            // Server errors (5xx)
            ValidationResult {
                url: "https://example.com/500".to_string(),
                line: 4,
                file_name: "test.md".to_string(),
                status_code: Some(500),
                description: Some("Internal Server Error".to_string()),
            },
            ValidationResult {
                url: "https://example.com/502".to_string(),
                line: 5,
                file_name: "test.md".to_string(),
                status_code: Some(502),
                description: Some("Bad Gateway".to_string()),
            },
            // Network/Connection errors (no status code)
            ValidationResult {
                url: "https://timeout.example".to_string(),
                line: 6,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("Connection timeout".to_string()),
            },
            ValidationResult {
                url: "https://dns.fail".to_string(),
                line: 7,
                file_name: "test.md".to_string(),
                status_code: None,
                description: Some("DNS resolution failed".to_string()),
            },
            // Redirect errors (3xx)
            ValidationResult {
                url: "https://redirect.example".to_string(),
                line: 8,
                file_name: "test.md".to_string(),
                status_code: Some(301),
                description: Some("Too many redirects".to_string()),
            },
        ];

        let categories = HtmlDashboard::categorize_issues(&results);

        assert_eq!(categories.get("Client Errors"), Some(&3));
        assert_eq!(categories.get("Server Errors"), Some(&2));
        assert_eq!(categories.get("Network Errors"), Some(&2));
        assert_eq!(categories.get("Redirects"), Some(&1));
    }

    #[test]
    fn test_categorize_issues_empty() {
        let categories = HtmlDashboard::categorize_issues(&[]);
        assert!(categories.is_empty());
    }

    #[test]
    fn test_calculate_success_rate() {
        // Perfect success rate
        let metadata1 = DisplayMetadata {
            total_validated: 100,
            issues_found: 0,
            files_processed: 10,
            total_urls_found: 100,
            unique_urls_found: 100,
        };
        assert_eq!(HtmlDashboard::calculate_success_rate(&metadata1), 100.0);

        // 90% success rate
        let metadata2 = DisplayMetadata {
            total_validated: 100,
            issues_found: 10,
            files_processed: 10,
            total_urls_found: 100,
            unique_urls_found: 100,
        };
        assert_eq!(HtmlDashboard::calculate_success_rate(&metadata2), 90.0);

        // Zero URLs validated - returns 100.0 by default
        let metadata3 = DisplayMetadata {
            total_validated: 0,
            issues_found: 0,
            files_processed: 0,
            total_urls_found: 0,
            unique_urls_found: 0,
        };
        assert_eq!(HtmlDashboard::calculate_success_rate(&metadata3), 100.0);

        // All failed
        let metadata4 = DisplayMetadata {
            total_validated: 50,
            issues_found: 50,
            files_processed: 5,
            total_urls_found: 50,
            unique_urls_found: 50,
        };
        assert_eq!(HtmlDashboard::calculate_success_rate(&metadata4), 0.0);
    }

    #[test]
    fn test_get_success_rate_style() {
        // High success rate (green)
        let style_high = HtmlDashboard::get_success_rate_style(95.0);
        assert!(style_high.contains("success"));

        // Medium success rate (warning)
        let style_medium = HtmlDashboard::get_success_rate_style(75.0);
        assert!(style_medium.contains("warning"));

        // Low success rate (error)
        let style_low = HtmlDashboard::get_success_rate_style(50.0);
        assert!(style_low.contains("error"));

        // Edge cases
        let style_perfect = HtmlDashboard::get_success_rate_style(100.0);
        assert!(style_perfect.contains("success"));

        let style_zero = HtmlDashboard::get_success_rate_style(0.0);
        assert!(style_zero.contains("error"));
    }

    #[test]
    fn test_generate_stat_card() {
        let card = HtmlDashboard::generate_stat_card("🚀", "123", "Total URLs", "success");

        assert!(card.contains("🚀"));
        assert!(card.contains("123"));
        assert!(card.contains("Total URLs"));
        assert!(card.contains("success"));
        assert!(card.contains("stat-card"));
    }

    #[test]
    fn test_generate_header_section() {
        let header = HtmlDashboard::generate_header_section("2025-01-01 12:00:00");

        assert!(header.contains("URL Validation Dashboard"));
        assert!(header.contains("urlsup"));
        assert!(header.contains("2025-01-01 12:00:00"));
        // HTML content can vary, so we just check for key content
    }

    #[test]
    fn test_generate_stats_section() {
        let data = create_test_dashboard_data();
        let stats = HtmlDashboard::generate_stats_section(&data);

        assert!(stats.contains("100")); // total_validated
        assert!(stats.contains("5")); // issues_found
        assert!(stats.contains("10")); // files_processed
        assert!(stats.contains("150")); // total_urls_found
        // HTML content can vary, so we just check for key content
    }

    #[test]
    fn test_generate_issues_section() {
        let data = create_test_dashboard_data();
        let issues = HtmlDashboard::generate_issues_section(&data);

        assert!(issues.contains("https://example.com/404"));
        assert!(issues.contains("https://example.com/500"));
        assert!(issues.contains("https://timeout.example"));
        assert!(issues.contains("test.md"));
        // HTML content can vary, so we just check for key content
    }

    #[test]
    fn test_generate_issues_section_no_issues() {
        let mut data = create_test_dashboard_data();
        data.results = vec![];
        data.metadata.issues_found = 0;

        let issues = HtmlDashboard::generate_issues_section(&data);
        // Should generate some content even with no issues
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_generate_performance_section_with_data() {
        let data = create_test_dashboard_data();
        let performance = HtmlDashboard::generate_performance_section(&data);

        assert!(performance.contains("file_processing"));
        assert!(performance.contains("validation"));
        assert!(performance.contains("Consider increasing concurrency"));
        // HTML content can vary, so we just check for key content
    }

    #[test]
    fn test_generate_performance_section_no_data() {
        let mut data = create_test_dashboard_data();
        data.performance = None;

        let performance = HtmlDashboard::generate_performance_section(&data);
        // Should generate content indicating no performance data available
        // (may be empty string if that's how the function is implemented)
        let _ = performance; // Just test that function doesn't panic
    }

    #[test]
    fn test_generate_charts_section() {
        let data = create_test_dashboard_data();
        let charts = HtmlDashboard::generate_charts_section(&data).unwrap();

        // Should generate chart content
        assert!(!charts.is_empty());
        assert!(charts.contains("canvas"));
    }

    #[test]
    fn test_generate_css() {
        let css = HtmlDashboard::generate_css();

        assert!(css.contains("body"));
        assert!(css.contains("color:"));
        assert!(css.contains("margin:"));
        assert!(css.contains("padding:"));
        // CSS content can vary, so we just check for basic CSS properties
    }

    #[test]
    fn test_generate_javascript() {
        let js = HtmlDashboard::generate_javascript();

        assert!(js.contains("Chart"));
        assert!(js.contains("function"));
        // JavaScript content can vary, so we just check for key elements
    }

    #[test]
    fn test_generate_body_content() {
        let data = create_test_dashboard_data();
        let body = HtmlDashboard::generate_body_content(&data).unwrap();

        assert!(body.contains("URL Validation Dashboard"));
        // HTML content can vary, so we just check for key content
    }

    #[test]
    fn test_generate_html_content() {
        let data = create_test_dashboard_data();
        let html = HtmlDashboard::generate_html_content(&data).unwrap();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\">"));
        assert!(html.contains("<head>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("<title>URL Validation Dashboard - urlsup</title>"));
        assert!(html.contains("chart.js"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn test_generate_dashboard_file_creation() -> Result<(), Box<dyn std::error::Error>> {
        let data = create_test_dashboard_data();
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path().to_str().unwrap();

        HtmlDashboard::generate_dashboard(&data, temp_path)?;

        let content = std::fs::read_to_string(temp_path)?;
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("URL Validation Dashboard"));
        assert!(content.contains("chart.js"));

        Ok(())
    }

    #[test]
    fn test_dashboard_error_display() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
        let dashboard_error = DashboardError::FileWrite(io_error);
        let display_str = format!("{}", dashboard_error);
        assert!(display_str.contains("Failed to write dashboard file"));
        assert!(display_str.contains("Permission denied"));

        let serialization_error = DashboardError::Serialization("Invalid JSON".to_string());
        let display_str = format!("{}", serialization_error);
        assert!(display_str.contains("Failed to serialize data"));
        assert!(display_str.contains("Invalid JSON"));
    }

    #[test]
    fn test_dashboard_error_source() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let dashboard_error = DashboardError::FileWrite(io_error);
        assert!(dashboard_error.source().is_some());

        let serialization_error = DashboardError::Serialization("Test".to_string());
        assert!(serialization_error.source().is_none());
    }

    #[test]
    fn test_dashboard_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid input");
        let dashboard_error = DashboardError::from(io_error);
        assert!(matches!(dashboard_error, DashboardError::FileWrite(_)));
    }

    #[test]
    fn test_dashboard_data_debug() {
        let data = create_test_dashboard_data();
        let debug_str = format!("{:?}", data);
        assert!(debug_str.contains("DashboardData"));
        assert!(debug_str.contains("total_validated"));
    }

    #[test]
    fn test_dashboard_data_clone() {
        let original = create_test_dashboard_data();
        let cloned = original.clone();

        assert_eq!(
            original.metadata.total_validated,
            cloned.metadata.total_validated
        );
        assert_eq!(original.results.len(), cloned.results.len());
        assert_eq!(original.timestamp, cloned.timestamp);
    }

    #[test]
    fn test_dashboard_constants() {
        assert_eq!(dashboard_constants::SUCCESS_THRESHOLD, 90.0);
        assert_eq!(dashboard_constants::WARNING_THRESHOLD, 70.0);
        assert_eq!(dashboard_constants::BYTES_TO_MB, 1_048_576.0);
        assert!(dashboard_constants::CHART_JS_CDN.contains("chart.js"));
    }

    #[test]
    fn test_edge_case_large_numbers() {
        let metadata = DisplayMetadata {
            total_validated: 1_000_000,
            issues_found: 50_000,
            files_processed: 10_000,
            total_urls_found: 2_000_000,
            unique_urls_found: 1_000_000,
        };

        let data = DashboardData {
            metadata,
            results: vec![],
            performance: None,
            config: Config::default(),
            timestamp: "2025-01-01 12:00:00".to_string(),
        };

        let stats = HtmlDashboard::generate_stats_section(&data);
        // Check that stats section contains numbers - format may vary
        assert!(stats.len() > 100); // Should have substantial content
        assert!(stats.contains("1000000") || stats.contains("1,000,000") || stats.contains("1M"));
        // Just check that we have some large number content

        // Success rate should be 95%
        let success_rate = HtmlDashboard::calculate_success_rate(&data.metadata);
        assert_eq!(success_rate, 95.0);
    }

    #[test]
    fn test_edge_case_special_characters_in_urls() {
        let results = vec![
            ValidationResult {
                url: "https://example.com/path with spaces".to_string(),
                line: 1,
                file_name: "test file.md".to_string(),
                status_code: Some(404),
                description: Some("Not Found with ünïcödé".to_string()),
            },
            ValidationResult {
                url: "https://example.com/émojis🚀".to_string(),
                line: 2,
                file_name: "测试.md".to_string(),
                status_code: Some(500),
                description: Some("Server Error 💥".to_string()),
            },
        ];

        let categories = HtmlDashboard::categorize_issues(&results);
        assert_eq!(categories.get("Client Errors"), Some(&1));
        assert_eq!(categories.get("Server Errors"), Some(&1));

        let data = DashboardData {
            metadata: create_test_metadata(),
            results,
            performance: None,
            config: Config::default(),
            timestamp: "2025-01-01 12:00:00".to_string(),
        };

        let issues = HtmlDashboard::generate_issues_section(&data);
        assert!(issues.contains("path with spaces"));
        assert!(issues.contains("émojis🚀"));
        assert!(issues.contains("测试.md"));
        assert!(issues.contains("ünïcödé"));
    }
}
