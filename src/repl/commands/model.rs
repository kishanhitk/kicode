use crate::config::Config;
use crate::error::Result;
use colored::Colorize;
use std::io::{self, BufRead, Write};

/// Curated list of popular coding-focused models: (id, display_name)
pub const POPULAR_MODELS: &[(&str, &str)] = &[
    ("x-ai/grok-code-fast-1", "Grok Code Fast 1 (Default)"),
    ("moonshotai/kimi-k2.5", "Kimi K2.5"),
    ("anthropic/claude-sonnet-4.5", "Claude Sonnet 4.5"),
    ("minimax/minimax-m2.1", "MiniMax M2.1"),
    ("anthropic/claude-opus-4.5", "Claude Opus 4.5"),
    ("google/gemini-3-flash-preview", "Gemini 3 Flash Preview"),
    ("z-ai/glm-4.7", "GLM 4.7"),
    ("openai/gpt-5.2", "GPT-5.2"),
    ("openai/gpt-5.2-codex", "GPT-5.2-Codex"),
];

/// Result of model selection
pub enum ModelSelection {
    /// User selected a new model (model_id, display_name)
    Selected(String, String),
    /// User selected the same model they already have
    AlreadyCurrent,
    /// User cancelled the selection
    Cancelled,
    /// Selection failed with an error
    Error(String),
}

/// Shows model selection menu.
/// This function reads from stdin and must be called from a blocking context.
pub fn show_model_menu(current_model: String) -> ModelSelection {
    println!("\n{}", "Model Selection".bold());
    println!("Current: {}\n", current_model.green());

    // Print numbered list
    for (i, (id, name)) in POPULAR_MODELS.iter().enumerate() {
        let marker = if *id == current_model { "*" } else { " " };
        println!("{} {}. {} ({})", marker, i + 1, name.cyan(), id.dimmed());
    }

    println!();
    print!("{}", "Enter number (or 'q' to cancel): ".yellow());
    io::stdout().flush().ok();

    // Read user input
    let stdin = io::stdin();
    let mut input = String::new();
    if stdin.lock().read_line(&mut input).is_err() {
        return ModelSelection::Error("Failed to read input".to_string());
    }

    let input = input.trim();

    // Handle cancel
    if input.eq_ignore_ascii_case("q") || input.is_empty() {
        return ModelSelection::Cancelled;
    }

    // Parse number
    match input.parse::<usize>() {
        Ok(n) if n >= 1 && n <= POPULAR_MODELS.len() => {
            let (new_model, name) = POPULAR_MODELS[n - 1];
            if new_model == current_model {
                ModelSelection::AlreadyCurrent
            } else {
                ModelSelection::Selected(new_model.to_string(), name.to_string())
            }
        }
        _ => ModelSelection::Error(format!("Invalid selection: {}", input)),
    }
}

/// Saves the selected model to config file
pub fn save_model(model: &str) -> Result<()> {
    let mut file_config = Config::load_file_config().unwrap_or_default();

    file_config.model = if model == Config::default_model() {
        None // Don't store default explicitly
    } else {
        Some(model.to_string())
    };

    file_config.save()
}
