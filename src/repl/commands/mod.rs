pub mod model;

/// Available slash commands with descriptions
pub const COMMANDS: &[(&str, &str)] = &[
    ("model", "Change AI model"),
    ("clear", "Clear conversation history"),
    ("help", "Show help"),
    ("exit", "Exit the program"),
];
