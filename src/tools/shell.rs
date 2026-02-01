use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Command;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "shell"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command and return its output. Use this for running programs, git commands, build tools, etc. Dangerous commands will require user confirmation."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' parameter"))?;

        let working_dir = args["working_dir"].as_str();

        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };

        let shell_arg = if cfg!(target_os = "windows") {
            "/C"
        } else {
            "-c"
        };

        let mut cmd = Command::new(shell);
        cmd.arg(shell_arg).arg(command);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        match cmd.output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let mut result = String::new();

                if !stdout.is_empty() {
                    result.push_str(&stdout);
                }

                if !stderr.is_empty() {
                    if !result.is_empty() {
                        result.push_str("\n--- stderr ---\n");
                    }
                    result.push_str(&stderr);
                }

                if result.is_empty() {
                    result = "(no output)".to_string();
                }

                // Truncate very long outputs
                if result.len() > 10000 {
                    result.truncate(10000);
                    result.push_str("\n... (output truncated)");
                }

                let success = output.status.success();
                if success {
                    Ok(ToolResult::success(result))
                } else {
                    Ok(ToolResult::error(format!(
                        "Command exited with code {}\n{}",
                        output.status.code().unwrap_or(-1),
                        result
                    )))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to execute command: {}", e))),
        }
    }
}
