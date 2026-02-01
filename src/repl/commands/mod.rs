pub mod model;

use dialoguer::{theme::ColorfulTheme, FuzzySelect};

/// Available slash commands with descriptions
pub const COMMANDS: &[(&str, &str)] = &[
    ("model", "Change AI model"),
    ("clear", "Clear conversation history"),
    ("help", "Show help"),
    ("exit", "Exit the program"),
];

/// Shows interactive command menu when user types "/"
/// Returns the selected command name, or None if cancelled
pub fn show_command_menu() -> Option<String> {
    let items: Vec<String> = COMMANDS
        .iter()
        .map(|(cmd, desc)| format!("/{:<10} {}", cmd, desc))
        .collect();

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select command")
        .items(&items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten();

    selection.map(|i| COMMANDS[i].0.to_string())
}
