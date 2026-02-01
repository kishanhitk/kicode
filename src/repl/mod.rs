pub mod commands;
pub mod output;

use crate::api::client::OpenRouterClient;
use crate::api::types::{Message, Role};
use crate::conversation::Conversation;
use crate::safety::analyzer::SafetyAnalyzer;
use crate::tools::ToolRegistry;
use anyhow::Result;
use std::io::{self, BufRead, Write};

pub struct Repl {
    client: OpenRouterClient,
    conversation: Conversation,
    tools: ToolRegistry,
    safety: SafetyAnalyzer,
}

impl Repl {
    pub fn new(client: OpenRouterClient, system_prompt: String) -> Self {
        Self {
            client,
            conversation: Conversation::new(system_prompt),
            tools: ToolRegistry::new(),
            safety: SafetyAnalyzer::new(),
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        output::print_welcome();

        loop {
            let prompt = output::print_prompt();
            print!("{}", prompt);
            io::stdout().flush()?;

            let line = tokio::task::spawn_blocking(|| {
                let stdin = io::stdin();
                let mut line = String::new();
                match stdin.lock().read_line(&mut line) {
                    Ok(0) => None, // EOF
                    Ok(_) => Some(line),
                    Err(_) => None,
                }
            })
            .await?;

            let line = match line {
                Some(l) => l,
                None => {
                    output::print_info("Goodbye!");
                    break;
                }
            };

            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Handle "/" alone -> show command menu
            if line == "/" {
                let cmd = tokio::task::spawn_blocking(commands::show_command_menu).await?;
                if let Some(cmd) = cmd {
                    self.execute_command(&cmd).await;
                }
                continue;
            }

            // Handle slash commands
            if let Some(cmd) = line.strip_prefix('/') {
                self.execute_command(cmd).await;
                continue;
            }

            // Regular message
            if let Err(e) = self.process_message(line).await {
                output::print_error(&e.to_string());
            }
        }

        Ok(())
    }

    async fn process_message(&mut self, user_input: &str) -> Result<()> {
        self.conversation.add_message(Message {
            role: Role::User,
            content: Some(user_input.to_string()),
            tool_calls: None,
            tool_call_id: None,
        });

        let mut after_tools = false;
        let mut retries = 0;
        loop {
            // Clear thinking indicator before streaming
            if after_tools {
                output::clear_thinking();
            }

            let messages = self.conversation.get_messages();
            let tool_schemas = self.tools.get_schemas();

            let response = self
                .client
                .chat_stream(messages, tool_schemas, |chunk| {
                    output::print_streaming(&chunk);
                })
                .await?;

            println!();

            // Check for empty response after tool calls (provider bug workaround)
            let is_empty = response
                .content
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(true)
                && response.tool_calls.is_none();

            if after_tools && is_empty && retries < 2 {
                // Empty response after tools - retry
                output::print_warning("Empty response, retrying...");
                retries += 1;
                continue;
            }
            retries = 0;

            self.conversation.add_message(response.clone());

            if let Some(ref tool_calls) = response.tool_calls {
                for tool_call in tool_calls {
                    output::print_tool_call(
                        &tool_call.function.name,
                        &tool_call.function.arguments,
                    );

                    let result = self
                        .execute_tool(&tool_call.function.name, &tool_call.function.arguments)
                        .await;

                    let result_content = match result {
                        Ok(r) => {
                            output::print_tool_result(&r.output);
                            r.output
                        }
                        Err(e) => {
                            let err_msg = format!("Error: {}", e);
                            output::print_error(&err_msg);
                            err_msg
                        }
                    };

                    self.conversation.add_message(Message {
                        role: Role::Tool,
                        content: Some(result_content),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    });
                }
                // Show thinking indicator before next API call
                output::print_thinking();
                after_tools = true;
            } else {
                break;
            }
        }

        Ok(())
    }

    async fn execute_tool(&self, name: &str, args_str: &str) -> Result<crate::tools::ToolResult> {
        let args: serde_json::Value = serde_json::from_str(args_str)?;

        if name == "shell" {
            if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                if self.safety.is_dangerous(command) {
                    if !self.confirm_execution(command).await? {
                        return Err(crate::error::KicodeError::CommandRejected.into());
                    }
                }
            }
        }

        self.tools.execute(name, args).await
    }

    async fn confirm_execution(&self, command: &str) -> Result<bool> {
        let prompt = output::print_confirm_prompt(command);
        print!("{}", prompt);
        io::stdout().flush()?;

        let response = tokio::task::spawn_blocking(|| {
            let stdin = io::stdin();
            let mut input = String::new();
            stdin.lock().read_line(&mut input).ok();
            input
        })
        .await?;

        Ok(response.trim().eq_ignore_ascii_case("y"))
    }

    /// Execute a slash command by name
    async fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "help" => output::print_help(),
            "exit" | "quit" => {
                output::print_info("Goodbye!");
                std::process::exit(0);
            }
            "clear" => {
                self.conversation.clear();
                output::print_info("Conversation cleared.");
            }
            "model" => {
                self.handle_model_command().await;
            }
            _ => {
                output::print_error(&format!("Unknown command: /{}", cmd));
                output::print_info("Type / to see available commands.");
            }
        }
    }

    /// Handle the /model command with interactive selection
    async fn handle_model_command(&mut self) {
        let current_model = self.client.model().to_string();

        // Run the interactive selection in a blocking context
        let selection = tokio::task::spawn_blocking(move || {
            commands::model::show_model_menu(current_model)
        })
        .await;

        let selection = match selection {
            Ok(s) => s,
            Err(e) => {
                output::print_error(&format!("Failed to show model menu: {}", e));
                return;
            }
        };

        match selection {
            commands::model::ModelSelection::Selected(model_id, name) => {
                // Update runtime
                self.client.set_model(model_id.clone());

                // Persist to config
                if let Err(e) = commands::model::save_model(&model_id) {
                    output::print_warning(&format!(
                        "Model changed to {} but failed to save: {}",
                        name, e
                    ));
                } else {
                    output::print_info(&format!("Model changed to: {} ({})", name, model_id));
                }
            }
            commands::model::ModelSelection::AlreadyCurrent => {
                output::print_info("Already using this model.");
            }
            commands::model::ModelSelection::Cancelled => {
                output::print_info("Model selection cancelled.");
            }
            commands::model::ModelSelection::Error(e) => {
                output::print_error(&e);
            }
        }
    }
}
