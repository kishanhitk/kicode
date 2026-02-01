use super::{Tool, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{Value, json};
use std::fs;
use walkdir::WalkDir;

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search for a pattern in files. Supports regex patterns. Returns matching lines with file paths and line numbers."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "The file or directory to search in (defaults to current directory)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "Optional glob pattern to filter files (e.g., '*.rs', '*.py')"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of matching lines to return (default: 50)"
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines to show before and after each match (default: 0)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let pattern_str = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'pattern' parameter"))?;

        let path = args["path"].as_str().unwrap_or(".");
        let file_pattern = args["file_pattern"].as_str();
        let max_results = args["max_results"].as_i64().unwrap_or(50) as usize;
        let context_lines = args["context_lines"].as_i64().unwrap_or(0) as usize;

        let regex = match Regex::new(pattern_str) {
            Ok(r) => r,
            Err(e) => return Ok(ToolResult::error(format!("Invalid regex pattern: {}", e))),
        };

        let file_regex = file_pattern
            .map(|p| {
                let glob_to_regex = p.replace('.', r"\.").replace('*', ".*").replace('?', ".");
                Regex::new(&format!("{}$", glob_to_regex)).ok()
            })
            .flatten();

        let mut results: Vec<String> = Vec::new();
        let mut total_matches = 0;

        let path_meta = fs::metadata(path);
        let is_file = path_meta.map(|m| m.is_file()).unwrap_or(false);

        if is_file {
            if let Some(matches) = search_file(path, &regex, context_lines) {
                for m in matches {
                    if results.len() >= max_results {
                        break;
                    }
                    results.push(m);
                    total_matches += 1;
                }
            }
        } else {
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if results.len() >= max_results {
                    break;
                }

                if !entry.file_type().is_file() {
                    continue;
                }

                let file_path = entry.path();
                let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip hidden files and directories
                if file_name.starts_with('.') {
                    continue;
                }

                // Skip binary files (common extensions)
                if is_likely_binary(file_name) {
                    continue;
                }

                // Apply file pattern filter
                if let Some(ref fr) = file_regex {
                    if !fr.is_match(file_name) {
                        continue;
                    }
                }

                let path_str = file_path.to_string_lossy();
                if let Some(matches) = search_file(&path_str, &regex, context_lines) {
                    for m in matches {
                        if results.len() >= max_results {
                            break;
                        }
                        results.push(m);
                        total_matches += 1;
                    }
                }
            }
        }

        if results.is_empty() {
            Ok(ToolResult::success("No matches found."))
        } else {
            let mut output = format!("Found {} matches:\n\n", total_matches);
            output.push_str(&results.join("\n"));
            if total_matches >= max_results {
                output.push_str(&format!("\n\n(showing first {} results)", max_results));
            }
            Ok(ToolResult::success(output))
        }
    }
}

fn search_file(path: &str, regex: &Regex, context_lines: usize) -> Option<Vec<String>> {
    let content = fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    let mut results = Vec::new();
    let mut last_printed_line = 0;

    for (i, line) in lines.iter().enumerate() {
        if regex.is_match(line) {
            let start = i.saturating_sub(context_lines);
            let end = (i + context_lines + 1).min(lines.len());

            // Add separator if there's a gap
            if start > last_printed_line && !results.is_empty() {
                results.push("--".to_string());
            }

            for j in start..end {
                if j >= last_printed_line {
                    let prefix = if j == i { ">" } else { " " };
                    results.push(format!("{}:{}:{} {}", path, j + 1, prefix, lines[j]));
                    last_printed_line = j + 1;
                }
            }
        }
    }

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

fn is_likely_binary(filename: &str) -> bool {
    let binary_extensions = [
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "pdf", "doc", "docx", "xls", "xlsx",
        "ppt", "pptx", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "exe", "dll", "so", "dylib",
        "a", "o", "obj", "wasm", "class", "pyc", "pyo", "mp3", "mp4", "avi", "mov", "mkv", "wav",
        "flac", "ttf", "otf", "woff", "woff2", "eot", "sqlite", "db",
    ];

    if let Some(ext) = filename.rsplit('.').next() {
        binary_extensions.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}
