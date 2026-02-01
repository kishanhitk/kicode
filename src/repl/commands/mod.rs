pub mod model;

use colored::Colorize;
use inquire::Select;

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
    println!();

    let items: Vec<String> = COMMANDS
        .iter()
        .map(|(cmd, desc)| format!("/{} - {}", cmd, desc))
        .collect();

    let items_refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();

    match Select::new("Select command:", items_refs).prompt() {
        Ok(selection) => {
            // Extract command name: "/model - ..." -> "model"
            selection
                .split_whitespace()
                .next()
                .map(|s: &str| s.trim_start_matches('/').to_string())
        }
        Err(_) => None,
    }
}
