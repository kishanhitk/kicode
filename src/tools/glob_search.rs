use super::{Tool, ToolResult};
use async_trait::async_trait;
use glob::glob;
use serde_json::{json, Value};
use std::path::PathBuf;

pub struct GlobSearchTool;

#[async_trait]
impl Tool for GlobSearchTool {
    fn name(&self) -> &'static str {
        "glob_search"
    }

    fn description(&self) -> &'static str {
        "Search for files matching a glob pattern. Use patterns like '**/*.rs' to find all Rust files, 'src/*.py' for Python files in src directory, etc."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The glob pattern to match files against (e.g., '**/*.rs', 'src/**/*.py')"
                },
                "base_path": {
                    "type": "string",
                    "description": "Optional base directory to search from (defaults to current directory)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;

        let base_path = args["base_path"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));

        let max_results = args["max_results"].as_i64().unwrap_or(100) as usize;

        let full_pattern = base_path.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();

        let mut matches: Vec<String> = Vec::new();

        match glob(&pattern_str) {
            Ok(paths) => {
                for entry in paths.take(max_results + 1) {
                    match entry {
                        Ok(path) => {
                            matches.push(path.display().to_string());
                        }
                        Err(e) => {
                            // Log but continue on individual path errors
                            eprintln!("Glob entry error: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                return Ok(ToolResult::error(format!("Invalid glob pattern: {}", e)));
            }
        }

        let truncated = matches.len() > max_results;
        if truncated {
            matches.truncate(max_results);
        }

        if matches.is_empty() {
            Ok(ToolResult::success("No files found matching the pattern."))
        } else {
            let mut output = format!("Found {} files:\n", matches.len());
            for path in &matches {
                output.push_str(path);
                output.push('\n');
            }
            if truncated {
                output.push_str(&format!("\n(showing first {} results)", max_results));
            }
            Ok(ToolResult::success(output))
        }
    }
}
