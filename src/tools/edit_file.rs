use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::fs;

pub struct EditFileTool;

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Edit a file by searching for a specific text and replacing it with new text. The search text must match exactly one location in the file to avoid ambiguity."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to the file to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "The exact text to search for and replace"
                },
                "new_text": {
                    "type": "string",
                    "description": "The text to replace it with"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'path' parameter"))?;
        let old_text = args["old_text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'old_text' parameter"))?;
        let new_text = args["new_text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'new_text' parameter"))?;

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::error(format!("Failed to read file: {}", e))),
        };

        // Count occurrences
        let matches: Vec<_> = content.match_indices(old_text).collect();

        match matches.len() {
            0 => Ok(ToolResult::error(
                "Text not found in file. Make sure old_text matches exactly, including whitespace and newlines.",
            )),
            1 => {
                let new_content = content.replacen(old_text, new_text, 1);

                if let Err(e) = fs::write(path, &new_content) {
                    return Ok(ToolResult::error(format!("Failed to write file: {}", e)));
                }

                // Show context around the change
                let match_pos = matches[0].0;
                let context = get_change_context(&new_content, match_pos, new_text.len());

                Ok(ToolResult::success(format!(
                    "Successfully edited {}.\n\nContext after edit:\n{}",
                    path, context
                )))
            }
            n => Ok(ToolResult::error(format!(
                "Ambiguous edit: found {} matches for the search text. Please provide more context to make the match unique.",
                n
            ))),
        }
    }
}

fn get_change_context(content: &str, pos: usize, new_len: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Find which line contains the position
    let mut current_pos = 0;
    let mut target_line = 0;

    for (i, line) in lines.iter().enumerate() {
        let line_len = line.len() + 1; // +1 for newline
        if current_pos + line_len > pos {
            target_line = i;
            break;
        }
        current_pos += line_len;
    }

    // Calculate approximate end line based on new text length
    let new_text_lines = new_len / 40; // rough estimate
    let end_line = (target_line + new_text_lines + 3).min(lines.len());
    let start_line = target_line.saturating_sub(2);

    lines[start_line..end_line]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", start_line + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n")
}
