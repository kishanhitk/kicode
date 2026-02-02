use anyhow::Result;
use clap::{Parser, Subcommand};
use kicode::api::client::OpenRouterClient;
use kicode::config::Config;
use kicode::error::KicodeError;
use kicode::repl::Repl;
use kicode::setup::{run_first_run_setup, run_setup_command};
use kicode::update::{state as update_state, UpdateChecker, UpdateInfo};
use tokio::sync::oneshot;

#[derive(Parser)]
#[command(name = "kicode")]
#[command(about = "AI-powered coding assistant", long_about = None)]
struct Cli {
    /// Model to use (e.g., anthropic/claude-3.5-sonnet, openai/gpt-4o)
    #[arg(short, long, global = true)]
    model: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure kicode (API key and settings)
    Setup,
}

const SYSTEM_PROMPT: &str = r#"You are Kicode, an AI coding assistant running in a terminal. You help users with programming tasks by reading, writing, and editing code files, running shell commands, and searching codebases.

You have access to the following tools:
- read_file: Read file contents with line numbers
- write_file: Create or overwrite files
- edit_file: Make precise edits using search-replace
- shell: Execute shell commands (dangerous commands require user confirmation)
- glob_search: Find files by pattern (e.g., **/*.rs)
- grep: Search file contents with regex

Guidelines:
1. Be concise and helpful. Focus on solving the user's problem.
2. When editing files, use edit_file with precise old_text that matches exactly one location.
3. Read files before editing to understand their structure.
4. For complex changes, break them into smaller edits.
5. When running shell commands, explain what they do if non-obvious.
6. If a task requires multiple steps, explain your plan briefly first.
7. Handle errors gracefully and suggest fixes.

Remember: You're running in the user's terminal with real file access. Be careful with destructive operations."#;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Handle explicit setup command
    if let Some(Commands::Setup) = cli.command {
        run_setup_command().await?;
        return Ok(());
    }

    // Try to load config, or run first-run setup if API key is missing
    let config = match Config::load(cli.model.clone()) {
        Ok(c) => c,
        Err(KicodeError::Config(msg)) if msg.contains("API key not found") => {
            // API key missing - run interactive setup
            run_first_run_setup().await?
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!("\nRun 'kicode setup' to configure.");
            std::process::exit(1);
        }
    };

    // Spawn background update check if enabled and due
    let update_rx: Option<oneshot::Receiver<Option<UpdateInfo>>> =
        if config.release.check_updates && update_state::should_check() {
            let (tx, rx) = oneshot::channel();
            let channel = config.release.channel;
            tokio::spawn(async move {
                let result = UpdateChecker::new().check(channel).await.ok().flatten();
                update_state::record_check();
                let _ = tx.send(result);
            });
            Some(rx)
        } else {
            None
        };

    let client = OpenRouterClient::new(&config);
    let mut repl = Repl::new(
        client,
        SYSTEM_PROMPT.to_string(),
        config.release.channel,
        update_rx,
    );

    repl.run().await
}
