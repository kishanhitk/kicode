pub mod edit_file;
pub mod glob_search;
pub mod grep;
pub mod read_file;
pub mod shell;
pub mod write_file;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::api::types::ToolSchema;

#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub success: bool,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
        }
    }

    pub fn error(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: false,
        }
    }
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<ToolResult>;
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };

        registry.register(Box::new(read_file::ReadFileTool));
        registry.register(Box::new(write_file::WriteFileTool));
        registry.register(Box::new(edit_file::EditFileTool));
        registry.register(Box::new(shell::ShellTool));
        registry.register(Box::new(glob_search::GlobSearchTool));
        registry.register(Box::new(grep::GrepTool));

        registry
    }

    fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get_schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .values()
            .map(|tool| ToolSchema {
                schema_type: "function".to_string(),
                function: crate::api::types::FunctionSchema {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters_schema(),
                },
            })
            .collect()
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<ToolResult> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(args).await,
            None => Ok(ToolResult::error(format!("Unknown tool: {}", name))),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
