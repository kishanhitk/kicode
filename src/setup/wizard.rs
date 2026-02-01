use crate::api::client::OpenRouterClient;
use crate::config::{Config, FileConfig};
use crate::error::{KicodeError, Result};
use crate::setup::input::{prompt, prompt_secret};
use colored::Colorize;

/// Runs the first-run setup wizard when no API key is configured.
/// Returns a Config on success, or an error if setup is cancelled.
pub async fn run_first_run_setup() -> Result<Config> {
    println!();
    println!("{}", "Welcome to Kicode!".bold().cyan());
    println!("{}", "Let's get you set up.".dimmed());
    println!();
    println!("To use Kicode, you need an OpenRouter API key.");
    println!(
        "Get one at: {}",
        "https://openrouter.ai/keys".underline().blue()
    );
    println!();

    // Load existing file config to preserve non-API settings (e.g., safety patterns)
    let existing_file_config = Config::load_file_config().unwrap_or_default();

    let api_key = prompt_api_key().await?;
    let model = prompt_model(existing_file_config.model.as_deref()).await?;

    // Save configuration, preserving existing settings like safety config
    let file_config = FileConfig {
        api_key: Some(api_key.clone()),
        model: if model == Config::default_model() {
            None
        } else {
            Some(model.clone())
        },
        safety: existing_file_config.safety.clone(),
    };

    file_config.save()?;

    let config_path = Config::config_path();
    println!();
    println!(
        "{} Config saved to {}",
        "".green(),
        config_path.display().to_string().dimmed()
    );
    println!();

    Ok(Config {
        api_key,
        model,
        safety: file_config.safety,
    })
}

/// Runs the explicit setup command for reconfiguration.
pub async fn run_setup_command() -> Result<()> {
    println!();
    println!("{}", "Kicode Setup".bold().cyan());
    println!();

    // Load existing file config (ignores env vars to show what's actually in the file)
    let existing = Config::load_file_config().ok();

    if let Some(ref file_config) = existing {
        if file_config.api_key.is_some() || file_config.model.is_some() {
            println!("{}", "Current configuration (from file):".dimmed());
            if let Some(ref key) = file_config.api_key {
                let masked_key = mask_api_key(key);
                println!("  API Key: {}", masked_key.yellow());
            }
            let model_display = file_config
                .model
                .as_deref()
                .unwrap_or(Config::default_model());
            println!("  Model:   {}", model_display.green());
            println!();
        }
    }

    // Prompt for new API key
    let current_key = existing.as_ref().and_then(|c| c.api_key.as_deref());
    let api_key = if current_key.is_some() {
        prompt_api_key_optional(current_key).await?
    } else {
        prompt_api_key().await?
    };

    // Prompt for model
    let current_model = existing.as_ref().and_then(|c| c.model.as_deref());
    let model = prompt_model(current_model).await?;

    // Save configuration, preserving existing settings like safety config
    let existing_safety = existing.map(|c| c.safety).unwrap_or_default();
    let file_config = FileConfig {
        api_key: Some(api_key),
        model: if model == Config::default_model() {
            None
        } else {
            Some(model)
        },
        safety: existing_safety,
    };

    file_config.save()?;

    println!();
    println!("{} Configuration updated!", "".green());
    println!();

    Ok(())
}

/// Prompts for API key with validation (required).
async fn prompt_api_key() -> Result<String> {
    loop {
        let key = prompt_secret("Enter your API key: ")
            .await
            .map_err(|e| KicodeError::Config(format!("Failed to read input: {}", e)))?;

        if key.is_empty() {
            println!("{}", "API key is required.".red());
            continue;
        }

        print!("{}", "Validating... ".dimmed());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match OpenRouterClient::validate_key(&key).await {
            Ok(true) => {
                println!("{}", "".green());
                return Ok(key);
            }
            Ok(false) => {
                println!("{}", "".red());
                println!(
                    "{}",
                    "Invalid API key. Please check your key and try again.".red()
                );
            }
            Err(e) => {
                println!("{}", "".red());
                println!("{} {}", "Network error:".red(), e);
                println!("{}", "Please check your internet connection.".dimmed());
            }
        }
    }
}

/// Prompts for API key with validation (optional, keeps current if empty).
async fn prompt_api_key_optional(current: Option<&str>) -> Result<String> {
    loop {
        let key = prompt_secret("Enter new API key (Enter to keep current): ")
            .await
            .map_err(|e| KicodeError::Config(format!("Failed to read input: {}", e)))?;

        if key.is_empty() {
            if let Some(current_key) = current {
                return Ok(current_key.to_string());
            }
            println!("{}", "API key is required.".red());
            continue;
        }

        print!("{}", "Validating... ".dimmed());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        match OpenRouterClient::validate_key(&key).await {
            Ok(true) => {
                println!("{}", "".green());
                return Ok(key);
            }
            Ok(false) => {
                println!("{}", "".red());
                println!(
                    "{}",
                    "Invalid API key. Please check your key and try again.".red()
                );
            }
            Err(e) => {
                println!("{}", "".red());
                println!("{} {}", "Network error:".red(), e);
                println!("{}", "Please check your internet connection.".dimmed());
            }
        }
    }
}

/// Prompts for model selection (optional, uses default if empty).
async fn prompt_model(current: Option<&str>) -> Result<String> {
    let default = Config::default_model();
    let prompt_text = if let Some(curr) = current {
        format!("Model (Enter for current: {}): ", curr)
    } else {
        format!("Model (Enter for default: {}): ", default)
    };

    let model = prompt(&prompt_text)
        .await
        .map_err(|e| KicodeError::Config(format!("Failed to read input: {}", e)))?;

    if model.is_empty() {
        Ok(current.unwrap_or(default).to_string())
    } else {
        Ok(model)
    }
}

/// Masks an API key for display (shows first 6 and last 4 chars).
fn mask_api_key(key: &str) -> String {
    if key.len() <= 10 {
        return "*".repeat(key.len());
    }
    format!("{}...{}", &key[..6], &key[key.len() - 4..])
}
