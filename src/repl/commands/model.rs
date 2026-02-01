use crate::api::client::OpenRouterClient;
use crate::config::Config;
use crate::error::Result;
use crate::repl::output;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

/// Curated list of popular coding-focused models: (id, display_name)
const POPULAR_MODELS: &[(&str, &str)] = &[
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

/// Handles the /model command - interactive model selection
pub fn handle(client: &mut OpenRouterClient) -> Result<()> {
    let current = client.model().to_string();

    // Build display items, marking current model
    let items: Vec<String> = POPULAR_MODELS
        .iter()
        .map(|(id, name)| {
            let marker = if *id == current { "* " } else { "  " };
            format!("{}{} ({})", marker, name, id)
        })
        .collect();

    // Find current model's index for default selection
    let default_idx = POPULAR_MODELS
        .iter()
        .position(|(id, _)| *id == current)
        .unwrap_or(0);

    println!("\n{}", "Model Selection".bold());
    println!("Current: {}\n", current.green());

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select model (type to filter, arrows to navigate)")
        .items(&items)
        .default(default_idx)
        .interact_opt();

    match selection {
        Ok(Some(idx)) => {
            let (new_model, name): (&str, &str) = POPULAR_MODELS[idx];

            if new_model == current {
                output::print_info("Already using this model.");
                return Ok(());
            }

            // Update runtime
            client.set_model(new_model.to_string());

            // Persist to config
            if let Err(e) = save_model(new_model) {
                output::print_warning(&format!(
                    "Model changed to {} but failed to save: {}",
                    name, e
                ));
            } else {
                output::print_info(&format!("Model changed to: {} ({})", name, new_model));
            }
        }
        Ok(None) => {
            output::print_info("Model selection cancelled.");
        }
        Err(e) => {
            output::print_error(&format!("Selection failed: {}", e));
        }
    }

    Ok(())
}

fn save_model(model: &str) -> Result<()> {
    let mut file_config = Config::load_file_config().unwrap_or_default();

    file_config.model = if model == Config::default_model() {
        None // Don't store default explicitly
    } else {
        Some(model.to_string())
    };

    file_config.save()
}
