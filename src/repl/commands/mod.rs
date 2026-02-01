pub mod model;
pub mod update;

/// Available slash commands with descriptions
pub const COMMANDS: &[(&str, &str)] = &[
    ("model", "Change AI model"),
    ("update", "Check for updates"),
    ("clear", "Clear conversation history"),
    ("help", "Show help"),
    ("exit", "Exit the program"),
];
