use crate::validator::ValidationResult;

use json_value_merge::Merge;
use serde_json::json;
use serde_json::Value;

use std::io;

pub trait FormatValidationResults {
    fn format(&self, results: &[ValidationResult]) -> io::Result<String>;
}

#[derive(Default)]
pub struct JsonFormatter;

impl FormatValidationResults for JsonFormatter {
    fn format(&self, results: &[ValidationResult]) -> io::Result<String> {
        let mut output: Value = serde_json::from_str(r#"{"results":[]}"#)?;

        results.iter().for_each(|vr| {
            let json = json!({
                "url": vr.url,
                "line": vr.line,
                "file_name": vr.file_name,
                "status_code": vr.status_code.unwrap_or(0),
                "description": vr.description.as_ref().unwrap_or(&"".to_string())
            });
            output
                .merge_in("/results", json)
                .expect("Unable to merge JSON values");
        });

        Ok(output.to_string())
    }
}
