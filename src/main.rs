use anyhow::Result;
use clap::Parser;
use kicode::api::client::OpenRouterClient;
use kicode::config::Config;
use kicode::repl::Repl;

#[derive(Parser)]
#[command(name = "kicode")]
#[command(about = "AI-powered coding assistant", long_about = None)]
struct Cli {
    /// Model to use (e.g., anthropic/claude-3.5-sonnet, openai/gpt-4o)
    #[arg(short, long)]
    model: Option<String>,
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

    let config = match Config::load(cli.model) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            eprintln!("\nSet OPENROUTER_API_KEY via:");
            eprintln!("  1. .env file (copy .env.example)");
            eprintln!("  2. Environment variable");
            eprintln!("  3. ~/.config/kicode/config.toml");
            std::process::exit(1);
        }
    };

    let client = OpenRouterClient::new(&config);
    let mut repl = Repl::new(client, SYSTEM_PROMPT.to_string());

    repl.run().await
}
