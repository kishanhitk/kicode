pub mod commands;
pub mod output;

use crate::api::client::OpenRouterClient;
use crate::api::types::{Message, Role};
use crate::conversation::Conversation;
use crate::safety::analyzer::SafetyAnalyzer;
use crate::tools::ToolRegistry;
use anyhow::Result;
use colored::Colorize;
use crossterm::cursor;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, Clear, ClearType};
use std::io::{self, BufRead, Write};

/// Result of reading user input
enum InputResult {
    /// User typed a line of text and pressed Enter
    Line(String),
    /// User selected a slash command from the menu
    Command(String),
    /// User pressed Ctrl+C or Ctrl+D - exit
    Exit,
}

/// Get commands that match the current input query
fn get_matching_commands(buffer: &str) -> Vec<(&'static str, &'static str)> {
    let query = buffer.strip_prefix('/').unwrap_or("");
    commands::COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(query))
        .copied()
        .collect()
}

/// Render the command suggestions overlay below the input line.
/// Only shows commands that match the current filter (no dimmed non-matches).
fn render_command_suggestions(buffer: &str, selected_idx: usize) -> io::Result<()> {
    let matches = get_matching_commands(buffer);

    // If no matches, don't show anything
    if matches.is_empty() {
        return Ok(());
    }

    // Save cursor position and move down
    crossterm::execute!(io::stdout(), cursor::SavePosition)?;

    // Only render matching commands
    for (idx, (name, desc)) in matches.iter().enumerate() {
        crossterm::execute!(
            io::stdout(),
            cursor::MoveToNextLine(1),
            Clear(ClearType::CurrentLine)
        )?;

        let is_selected = idx == selected_idx;
        let prefix = if is_selected { "▸" } else { " " };

        if is_selected {
            // Highlighted: green and bold
            print!(
                "  {} /{} - {}",
                prefix.green(),
                name.green().bold(),
                desc.green()
            );
        } else {
            // Matching but not selected
            print!("  {} /{} - {}", prefix, name.cyan(), desc);
        }
    }

    // Restore cursor position
    crossterm::execute!(io::stdout(), cursor::RestorePosition)?;
    io::stdout().flush()?;

    Ok(())
}

/// Clear the command suggestions overlay.
/// Takes the number of lines to clear (number of previously shown matches).
fn clear_command_suggestions(num_lines: usize) -> io::Result<()> {
    crossterm::execute!(io::stdout(), cursor::SavePosition)?;

    for _ in 0..num_lines {
        crossterm::execute!(
            io::stdout(),
            cursor::MoveToNextLine(1),
            Clear(ClearType::CurrentLine)
        )?;
    }

    crossterm::execute!(io::stdout(), cursor::RestorePosition)?;
    io::stdout().flush()?;

    Ok(())
}

/// Read user input character by character with inline command filtering.
/// When input starts with '/', shows filtered command suggestions below the input.
fn read_input() -> io::Result<InputResult> {
    let mut buffer = String::new();
    let mut selected_idx: usize = 0;
    let mut prev_match_count: usize = 0; // Track how many lines were rendered

    terminal::enable_raw_mode()?;

    let result = loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
            {
                match (code, modifiers) {
                    // Ctrl+C or Ctrl+D -> exit
                    (KeyCode::Char('c'), KeyModifiers::CONTROL)
                    | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        if buffer.starts_with('/') {
                            clear_command_suggestions(prev_match_count)?;
                        }
                        print!("\r\n");
                        break InputResult::Exit;
                    }

                    // Enter -> submit line or select command
                    (KeyCode::Enter, _) => {
                        if buffer.starts_with('/') {
                            let matches = get_matching_commands(&buffer);
                            clear_command_suggestions(prev_match_count)?;

                            if !matches.is_empty() {
                                // Select the highlighted command
                                let (name, _) = matches[selected_idx];

                                // Update display to show full command before executing
                                let partial_len = buffer.len();
                                for _ in 0..partial_len {
                                    print!("\x08 \x08");
                                }
                                let full_cmd = format!("/{}", name);
                                print!("{}", full_cmd);
                                io::stdout().flush()?;

                                print!("\r\n");
                                break InputResult::Command(name.to_string());
                            } else {
                                // No matches - send as regular input to AI
                                print!("\r\n");
                                break InputResult::Line(buffer);
                            }
                        } else {
                            print!("\r\n");
                            break InputResult::Line(buffer);
                        }
                    }

                    // Up arrow - move selection up (only in command mode)
                    (KeyCode::Up, _) if buffer.starts_with('/') => {
                        let matches = get_matching_commands(&buffer);
                        if !matches.is_empty() && selected_idx > 0 {
                            selected_idx -= 1;
                            clear_command_suggestions(prev_match_count)?;
                            render_command_suggestions(&buffer, selected_idx)?;
                            prev_match_count = matches.len();
                        }
                    }

                    // Down arrow - move selection down (only in command mode)
                    (KeyCode::Down, _) if buffer.starts_with('/') => {
                        let matches = get_matching_commands(&buffer);
                        if selected_idx + 1 < matches.len() {
                            selected_idx += 1;
                            clear_command_suggestions(prev_match_count)?;
                            render_command_suggestions(&buffer, selected_idx)?;
                            prev_match_count = matches.len();
                        }
                    }

                    // Regular character
                    (KeyCode::Char(c), _) => {
                        buffer.push(c);
                        print!("{}", c);
                        io::stdout().flush()?;

                        // If in command mode, reset selection and re-render
                        if buffer.starts_with('/') {
                            selected_idx = 0; // Reset to first match
                            clear_command_suggestions(prev_match_count)?;
                            let matches = get_matching_commands(&buffer);
                            prev_match_count = matches.len();
                            render_command_suggestions(&buffer, selected_idx)?;
                        }
                    }

                    // Backspace
                    (KeyCode::Backspace, _) => {
                        let was_command_mode = buffer.starts_with('/');
                        if buffer.pop().is_some() {
                            // Move cursor back, overwrite with space, move back again
                            print!("\x08 \x08");
                            io::stdout().flush()?;

                            if buffer.starts_with('/') {
                                // Still in command mode - update suggestions
                                selected_idx = 0;
                                clear_command_suggestions(prev_match_count)?;
                                let matches = get_matching_commands(&buffer);
                                prev_match_count = matches.len();
                                render_command_suggestions(&buffer, selected_idx)?;
                            } else if was_command_mode {
                                // Exited command mode - clear suggestions
                                clear_command_suggestions(prev_match_count)?;
                                prev_match_count = 0;
                            }
                        }
                    }

                    // Escape -> clear line and suggestions
                    (KeyCode::Esc, _) => {
                        if buffer.starts_with('/') {
                            clear_command_suggestions(prev_match_count)?;
                            prev_match_count = 0;
                        }
                        // Clear the current line
                        for _ in 0..buffer.len() {
                            print!("\x08 \x08");
                        }
                        io::stdout().flush()?;
                        buffer.clear();
                        selected_idx = 0;
                    }

                    _ => {}
                }
            }
        }
    };

    terminal::disable_raw_mode()?;
    Ok(result)
}

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

            let input = tokio::task::spawn_blocking(read_input).await?;

            match input {
                Ok(InputResult::Exit) => {
                    output::print_info("Goodbye!");
                    break;
                }
                Ok(InputResult::Command(cmd)) => {
                    // User selected a command from the inline menu
                    if self.execute_command(&cmd).await {
                        break;
                    }
                }
                Ok(InputResult::Line(line)) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    // Handle slash commands (e.g., "/help", "/model")
                    if let Some(cmd) = line.strip_prefix('/') {
                        // Only treat as command if it matches a known command
                        let cmd_name = cmd.split_whitespace().next().unwrap_or("");
                        if commands::COMMANDS.iter().any(|(name, _)| *name == cmd_name)
                            || cmd_name == "quit"
                        {
                            if self.execute_command(cmd).await {
                                break;
                            }
                            continue;
                        }
                        // Not a known command - fall through to process_message
                    }

                    // Regular message
                    if let Err(e) = self.process_message(line).await {
                        output::print_error(&e.to_string());
                    }
                }
                Err(e) => {
                    output::print_error(&format!("Input error: {}", e));
                }
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

    /// Execute a slash command by name.
    /// Returns `true` if the REPL should exit.
    async fn execute_command(&mut self, cmd: &str) -> bool {
        match cmd {
            "help" => output::print_help(),
            "exit" | "quit" => {
                output::print_info("Goodbye!");
                return true;
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
        false
    }

    /// Handle the /model command with interactive selection
    async fn handle_model_command(&mut self) {
        let current_model = self.client.model().to_string();

        // Run the selection in a blocking context (reads from stdin)
        let selection =
            tokio::task::spawn_blocking(move || commands::model::show_model_menu(current_model))
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
