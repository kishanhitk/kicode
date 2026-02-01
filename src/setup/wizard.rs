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

    let api_key = prompt_api_key().await?;
    let model = prompt_model(None).await?;

    // Save configuration
    let file_config = FileConfig {
        api_key: Some(api_key.clone()),
        model: if model == Config::default_model() {
            None
        } else {
            Some(model.clone())
        },
        ..Default::default()
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

    // Try to load existing config
    let existing = Config::load(None).ok();

    if let Some(ref config) = existing {
        println!("{}", "Current configuration:".dimmed());
        let masked_key = mask_api_key(&config.api_key);
        println!("  API Key: {}", masked_key.yellow());
        println!("  Model:   {}", config.model.green());
        println!();
    }

    // Prompt for new API key
    let api_key = if existing.is_some() {
        prompt_api_key_optional(existing.as_ref().map(|c| c.api_key.as_str())).await?
    } else {
        prompt_api_key().await?
    };

    // Prompt for model
    let current_model = existing.as_ref().map(|c| c.model.as_str());
    let model = prompt_model(current_model).await?;

    // Save configuration
    let file_config = FileConfig {
        api_key: Some(api_key),
        model: if model == Config::default_model() {
            None
        } else {
            Some(model)
        },
        ..Default::default()
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
