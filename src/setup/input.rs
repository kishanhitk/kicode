use std::io::{self, BufRead, Write};

/// Prompts the user for text input.
/// Returns the trimmed input string.
pub async fn prompt(message: &str) -> io::Result<String> {
    print!("{}", message);
    io::stdout().flush()?;

    tokio::task::spawn_blocking(|| {
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        Ok(line.trim().to_string())
    })
    .await
    .map_err(io::Error::other)?
}

/// Prompts the user for secret input (password-style, masked).
/// Uses rpassword for secure input that doesn't echo to terminal.
pub async fn prompt_secret(message: &str) -> io::Result<String> {
    print!("{}", message);
    io::stdout().flush()?;

    tokio::task::spawn_blocking(rpassword::read_password)
        .await
        .map_err(io::Error::other)?
}

/// Prompts the user for confirmation (y/n).
/// Returns true for 'y' or 'Y', false otherwise.
/// If input is empty, returns the default value.
pub async fn confirm(message: &str, default: bool) -> io::Result<bool> {
    let suffix = if default { " [Y/n]: " } else { " [y/N]: " };
    let full_message = format!("{}{}", message, suffix);

    let response = prompt(&full_message).await?;

    if response.is_empty() {
        return Ok(default);
    }

    Ok(response.eq_ignore_ascii_case("y") || response.eq_ignore_ascii_case("yes"))
}
