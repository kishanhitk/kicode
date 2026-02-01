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
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use walkdir::WalkDir;

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

struct FileIndex {
    files: Vec<String>,
    files_lower: Vec<String>,
}

fn find_repo_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let fallback = dir.clone();
    loop {
        if dir.join(".git").exists() {
            return dir;
        }
        if !dir.pop() {
            return fallback;
        }
    }
}

fn is_excluded_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | "out"
            | ".next"
            | ".cache"
            | "vendor"
    )
}

fn build_file_index() -> FileIndex {
    let root = find_repo_root();
    let mut files = Vec::new();

    let walker = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy();
                !is_excluded_dir(&name)
            } else {
                true
            }
        });

    for entry in walker.filter_map(|entry| entry.ok()) {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(&root) {
                files.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    files.sort();
    let files_lower = files.iter().map(|p| p.to_ascii_lowercase()).collect();
    FileIndex { files, files_lower }
}

fn get_file_index() -> &'static FileIndex {
    static INDEX: OnceLock<FileIndex> = OnceLock::new();
    INDEX.get_or_init(build_file_index)
}

fn get_matching_files(query: &str, limit: usize) -> Vec<String> {
    let index = get_file_index();
    if limit == 0 {
        return Vec::new();
    }
    if query.is_empty() {
        return index.files.iter().take(limit).cloned().collect();
    }
    let needle = query.to_ascii_lowercase();
    index
        .files
        .iter()
        .zip(index.files_lower.iter())
        .filter(|(_, lower)| lower.contains(&needle))
        .take(limit)
        .map(|(path, _)| path.clone())
        .collect()
}

fn active_mention_query(buffer: &str) -> Option<String> {
    let at_pos = buffer.rfind('@')?;
    if at_pos > 0 {
        if let Some(prev) = buffer[..at_pos].chars().rev().next() {
            if prev.is_ascii_alphanumeric() || prev == '_' {
                return None;
            }
        }
    }
    let after = &buffer[at_pos + 1..];
    if after.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    Some(after.to_string())
}

fn render_file_suggestions(matches: &[String]) -> io::Result<()> {
    if matches.is_empty() {
        return Ok(());
    }

    crossterm::execute!(io::stdout(), cursor::SavePosition)?;

    for path in matches {
        crossterm::execute!(
            io::stdout(),
            cursor::MoveToNextLine(1),
            Clear(ClearType::CurrentLine)
        )?;
        print!("  @{}", path.cyan());
    }

    crossterm::execute!(io::stdout(), cursor::RestorePosition)?;
    io::stdout().flush()?;

    Ok(())
}

fn update_file_suggestions(buffer: &str, prev_count: &mut usize) -> io::Result<()> {
    if let Some(query) = active_mention_query(buffer) {
        let matches = get_matching_files(&query, 5);
        if *prev_count > 0 {
            clear_command_suggestions(*prev_count)?;
        }
        if !matches.is_empty() {
            render_file_suggestions(&matches)?;
        }
        *prev_count = matches.len();
    } else if *prev_count > 0 {
        clear_command_suggestions(*prev_count)?;
        *prev_count = 0;
    }
    Ok(())
}

fn extract_file_mentions(input: &str) -> Vec<String> {
    let pattern = Regex::new(r"(^|[^\w])@([^\s]+)").unwrap();
    pattern
        .captures_iter(input)
        .filter_map(|cap| cap.get(2))
        .map(|m| {
            m.as_str().trim_end_matches(|c: char| {
                matches!(
                    c,
                    '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}' | '>' | '"' | '\''
                )
            })
        })
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn resolve_mention_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() || candidate.exists() {
        return candidate;
    }
    find_repo_root().join(path)
}

fn format_file_with_line_numbers(content: &str) -> String {
    if content.is_empty() {
        return "(empty file)".to_string();
    }
    content
        .lines()
        .enumerate()
        .map(|(i, line)| format!("{:4} | {}", i + 1, line))
        .collect::<Vec<String>>()
        .join("\n")
}

fn expand_file_mentions(input: &str) -> (String, Vec<String>) {
    let mentions = extract_file_mentions(input);
    if mentions.is_empty() {
        return (input.to_string(), Vec::new());
    }

    let mut seen = HashSet::new();
    let mut sections = Vec::new();
    let mut warnings = Vec::new();

    for mention in mentions {
        let resolved = resolve_mention_path(&mention);
        let key = resolved.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        if resolved.is_dir() {
            warnings.push(format!("Skipped directory mention: {}", mention));
            continue;
        }
        match fs::read_to_string(&resolved) {
            Ok(content) => {
                let formatted = format_file_with_line_numbers(&content);
                sections.push(format!("File: {}\n{}", mention, formatted));
            }
            Err(e) => warnings.push(format!("Failed to read {}: {}", mention, e)),
        }
    }

    if sections.is_empty() {
        return (input.to_string(), warnings);
    }

    let mut expanded = input.to_string();
    expanded.push_str("\n\nReferenced files:\n");
    expanded.push_str(&sections.join("\n\n"));
    (expanded, warnings)
}

/// Read user input character by character with inline command filtering.
/// When input starts with '/', shows filtered command suggestions below the input.
fn read_input() -> io::Result<InputResult> {
    let mut buffer = String::new();
    let mut selected_idx: usize = 0;
    let mut prev_match_count: usize = 0; // Track how many lines were rendered
    let mut prev_file_match_count: usize = 0;

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
                        if prev_file_match_count > 0 {
                            clear_command_suggestions(prev_file_match_count)?;
                            prev_file_match_count = 0;
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
                            if prev_file_match_count > 0 {
                                clear_command_suggestions(prev_file_match_count)?;
                                prev_file_match_count = 0;
                            }
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
                            if prev_file_match_count > 0 {
                                clear_command_suggestions(prev_file_match_count)?;
                                prev_file_match_count = 0;
                            }
                            selected_idx = 0; // Reset to first match
                            clear_command_suggestions(prev_match_count)?;
                            let matches = get_matching_commands(&buffer);
                            prev_match_count = matches.len();
                            render_command_suggestions(&buffer, selected_idx)?;
                        } else {
                            update_file_suggestions(&buffer, &mut prev_file_match_count)?;
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
                            if !buffer.starts_with('/') {
                                update_file_suggestions(&buffer, &mut prev_file_match_count)?;
                            }
                        }
                    }

                    // Escape -> clear line and suggestions
                    (KeyCode::Esc, _) => {
                        if buffer.starts_with('/') {
                            clear_command_suggestions(prev_match_count)?;
                            prev_match_count = 0;
                        }
                        if prev_file_match_count > 0 {
                            clear_command_suggestions(prev_file_match_count)?;
                            prev_file_match_count = 0;
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
        let (expanded_input, warnings) = expand_file_mentions(user_input);
        for warning in warnings {
            output::print_warning(&warning);
        }

        self.conversation.add_message(Message {
            role: Role::User,
            content: Some(expanded_input),
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
