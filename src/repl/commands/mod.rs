pub mod model;

use colored::Colorize;
use std::io::{self, BufRead, Write};

/// Available slash commands with descriptions
pub const COMMANDS: &[(&str, &str)] = &[
    ("model", "Change AI model"),
    ("clear", "Clear conversation history"),
    ("help", "Show help"),
    ("exit", "Exit the program"),
];

/// Shows command menu when user types "/"
/// Returns the selected command name, or None if cancelled
pub fn show_command_menu() -> Option<String> {
    println!("\n{}", "Commands".bold());

    for (i, (cmd, desc)) in COMMANDS.iter().enumerate() {
        println!("  {}. /{} - {}", i + 1, cmd.green(), desc);
    }

    println!();
    print!("{}", "Enter number (or 'q' to cancel): ".yellow());
    io::stdout().flush().ok();

    let stdin = io::stdin();
    let mut input = String::new();
    if stdin.lock().read_line(&mut input).is_err() {
        return None;
    }

    let input = input.trim();

    if input.eq_ignore_ascii_case("q") || input.is_empty() {
        return None;
    }

    match input.parse::<usize>() {
        Ok(n) if n >= 1 && n <= COMMANDS.len() => Some(COMMANDS[n - 1].0.to_string()),
        _ => {
            println!("{}", "Invalid selection".red());
            None
        }
    }
}
