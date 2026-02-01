use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::fs;

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file at the specified path. Returns the file content with line numbers."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to read"
                },
                "start_line": {
                    "type": "integer",
                    "description": "Optional starting line number (1-indexed)"
                },
                "end_line": {
                    "type": "integer",
                    "description": "Optional ending line number (1-indexed, inclusive)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::error(format!("Failed to read file: {}", e))),
        };

        let lines: Vec<&str> = content.lines().collect();
        let start = args["start_line"]
            .as_i64()
            .map(|n| (n as usize).saturating_sub(1))
            .unwrap_or(0);
        let end = args["end_line"]
            .as_i64()
            .map(|n| n as usize)
            .unwrap_or(lines.len());

        let selected_lines: Vec<String> = lines
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start)
            .map(|(i, line)| format!("{:4} | {}", i + 1, line))
            .collect();

        let output = if selected_lines.is_empty() {
            "(empty file)".to_string()
        } else {
            selected_lines.join("\n")
        };

        Ok(ToolResult::success(output))
    }
}
