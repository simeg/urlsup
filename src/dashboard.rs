use crate::config::Config;
use crate::output::DisplayMetadata;
use crate::performance::PerformanceReport;
use crate::validator::ValidationResult;
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
}
