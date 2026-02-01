use crate::config::Config;
use crate::error::Result;
use colored::Colorize;
use inquire::Select;

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

/// Shows model selection menu with arrow-key navigation.
/// This function reads from stdin and must be called from a blocking context.
pub fn show_model_menu(current_model: String) -> ModelSelection {
    println!("\n{}", "Model Selection".bold());
    println!("Current: {}\n", current_model.green());

    // Build menu items with current model marked
    let items: Vec<String> = POPULAR_MODELS
        .iter()
        .map(|(id, name)| {
            let marker = if *id == current_model { "* " } else { "  " };
            format!("{}{} ({})", marker, name, id)
        })
        .collect();

    let items_refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();

    // Find starting cursor position (current model)
    let default_idx = POPULAR_MODELS
        .iter()
        .position(|(id, _)| *id == current_model)
        .unwrap_or(0);

    match Select::new("Select model:", items_refs)
        .with_starting_cursor(default_idx)
        .prompt()
    {
        Ok(selection) => {
            // Find which model was selected by matching display name
            let idx = POPULAR_MODELS
                .iter()
                .position(|(_, name): &(&str, &str)| selection.contains(name))
                .unwrap_or(0);

            let (new_model, name) = POPULAR_MODELS[idx];

            if new_model == current_model {
                ModelSelection::AlreadyCurrent
            } else {
                ModelSelection::Selected(new_model.to_string(), name.to_string())
            }
        }
        Err(_) => ModelSelection::Cancelled,
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
