use crate::config::{ReleaseChannel, VERSION};
use crate::update::UpdateChecker;
use colored::Colorize;

/// Check for updates and display the result
pub async fn check_for_updates(channel: ReleaseChannel) {
    println!("{} Checking for updates...", "Info:".blue());

    let checker = UpdateChecker::new();

    match checker.check(channel).await {
        Ok(Some(info)) => {
            let version_type = if info.is_prerelease {
                " (pre-release)".dimmed().to_string()
            } else {
                String::new()
            };

            println!();
            println!(
                "{} {} → {}{}",
                "Update available:".green().bold(),
                format!("v{}", info.current_version).dimmed(),
                format!("v{}", info.latest_version).green().bold(),
                version_type
            );
            println!();

            // Show install command with correct channel
            let install_cmd = if info.channel == ReleaseChannel::Beta {
                "KICODE_CHANNEL=beta curl -fsSL https://kicode.kishans.in/install | sh"
            } else {
                "curl -fsSL https://kicode.kishans.in/install | sh"
            };

            println!("{}", "To update, run:".dimmed());
            println!("  {}", install_cmd.cyan());
            println!();
            println!(
                "{} {}",
                "Release notes:".dimmed(),
                info.release_url.blue().underline()
            );
        }
        Ok(None) => {
            println!(
                "{} You're running the latest version (v{})",
                "Up to date:".green().bold(),
                VERSION
            );
        }
        Err(e) => {
            println!(
                "{} Failed to check for updates: {}",
                "Error:".red().bold(),
                e
            );
        }
    }
}
