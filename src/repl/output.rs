use colored::Colorize;

pub fn print_assistant(text: &str) {
    println!("{}", text.cyan());
}

pub fn print_streaming(text: &str) {
    use std::io::{self, Write};
    print!("{}", text.cyan());
    io::stdout().flush().ok();
}

pub fn print_tool_call(name: &str, args: &str) {
    println!(
        "{} {} {}",
        "Tool:".yellow().bold(),
        name.yellow(),
        args.dimmed()
    );
}

pub fn print_tool_result(result: &str) {
    let preview = if result.len() > 200 {
        format!("{}...", &result[..200])
    } else {
        result.to_string()
    };
    println!("{} {}", "Result:".green(), preview.dimmed());
}

pub fn print_error(msg: &str) {
    eprintln!("{} {}", "Error:".red().bold(), msg.red());
}

pub fn print_warning(msg: &str) {
    println!("{} {}", "Warning:".yellow().bold(), msg.yellow());
}

pub fn print_info(msg: &str) {
    println!("{} {}", "Info:".blue(), msg);
}

pub fn print_thinking() {
    use std::io::{self, Write};
    print!("{}", "Thinking...".dimmed());
    io::stdout().flush().ok();
}

pub fn clear_thinking() {
    use std::io::{self, Write};
    print!("\r{}\r", " ".repeat(20));
    io::stdout().flush().ok();
}

pub fn print_prompt() -> String {
    format!("{} ", ">>".green().bold())
}

pub fn print_confirm_prompt(command: &str) -> String {
    format!(
        "{} {} {}\n{} ",
        "Execute:".yellow().bold(),
        command.white(),
        "[y/N]".dimmed(),
        ">".yellow()
    )
}

pub fn print_help() {
    println!(
        r#"
{}

{}
  {}       - Show command menu (arrow-key navigation)
  {}    - Show this help
  {}   - Change AI model
  {}  - Check for updates
  {}   - Clear conversation history
  {}    - Exit the program

{}
  Just type your message and press Enter.
  The AI can read, write, edit files and run shell commands.
  Dangerous commands will require confirmation.
"#,
        "Kicode - AI Coding Assistant".cyan().bold(),
        "Commands:".yellow(),
        "/".green(),
        "/help".green(),
        "/model".green(),
        "/update".green(),
        "/clear".green(),
        "/exit".green(),
        "Usage:".yellow()
    );
}

pub fn print_welcome() {
    println!(
        "\n{}\n{}\n",
        "Welcome to Kicode!".cyan().bold(),
        "Type /help for commands, or start chatting.".dimmed()
    );
}

pub fn print_update_available(current: &str, latest: &str) {
    println!(
        "{} {} {} {}",
        "Update:".yellow(),
        format!("v{}", latest).green().bold(),
        "available".dimmed(),
        format!("(current: v{}, run /update for details)", current).dimmed()
    );
    println!();
}
