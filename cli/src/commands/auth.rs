use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

use crate::api_client::ApiClient;
use crate::ui;

fn credentials_path() -> String {
    dirs::home_dir()
        .map(|home| {
            home.join(".hyperstack")
                .join("credentials.toml")
                .display()
                .to_string()
        })
        .unwrap_or_else(|| "~/.hyperstack/credentials.toml".to_string())
}

pub fn login(api_key: Option<String>) -> Result<()> {
    let api_key = if let Some(key) = api_key {
        key
    } else {
        println!("{}", "Login to Hyperstack".bold());
        println!();
        print!("API Key: ");
        io::stdout().flush()?;

        let mut key = String::new();
        io::stdin().read_line(&mut key)?;
        key.trim().to_string()
    };

    if api_key.is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    // Save the key
    ApiClient::save_api_key(&api_key)?;

    // Verify the key works
    let spinner = ui::create_spinner("Verifying API key...");
    let client = ApiClient::new()?;

    match client.list_specs() {
        Ok(_) => {
            spinner.finish_and_clear();
            ui::print_success("API key saved and verified!");
            println!();
            println!("  Credentials: {}", credentials_path().dimmed());
            println!();
            println!("You are now ready to use Hyperstack!");
        }
        Err(e) => {
            spinner.finish_and_clear();
            // Remove invalid key
            let _ = ApiClient::delete_api_key();
            anyhow::bail!("Invalid API key: {}", e);
        }
    }

    Ok(())
}

pub fn logout() -> Result<()> {
    let spinner = ui::create_spinner("Logging out...");

    ApiClient::delete_api_key()?;

    spinner.finish_and_clear();
    ui::print_success("Logged out successfully");
    println!("  Your credentials have been removed from this device.");

    Ok(())
}

pub fn status() -> Result<()> {
    match ApiClient::load_api_key() {
        Ok(_) => {
            println!(
                "{} {}",
                ui::symbols::SUCCESS.green().bold(),
                "Authenticated".green().bold()
            );
            println!();
            println!("  You are logged in and ready to use Hyperstack.");
            println!("  Credentials: {}", credentials_path().dimmed());
            println!();
            println!(
                "  Run {} to verify with the server.",
                "hs auth whoami".cyan()
            );
        }
        Err(_) => {
            println!(
                "{} {}",
                ui::symbols::FAILURE.red().bold(),
                "Not authenticated".red().bold()
            );
            println!();
            println!("  Run {} to authenticate.", "hs auth login".cyan());
        }
    }

    Ok(())
}

pub fn whoami() -> Result<()> {
    let api_key = match ApiClient::load_api_key() {
        Ok(key) => key,
        Err(_) => {
            ui::print_error("Not authenticated");
            println!();
            println!("  Run {} to authenticate.", "hs auth login".cyan());
            return Ok(());
        }
    };

    let spinner = ui::create_spinner("Verifying authentication...");
    let client = ApiClient::new()?;

    match client.list_specs() {
        Ok(specs) => {
            spinner.finish_and_clear();
            println!(
                "{} {}",
                ui::symbols::SUCCESS.green().bold(),
                "Authenticated".green().bold()
            );
            println!();
            println!(
                "  API key: {}...{}",
                &api_key[..8.min(api_key.len())],
                &api_key[api_key.len().saturating_sub(4)..]
            );
            println!("  Stacks: {}", specs.len());
            println!("  Credentials: {}", credentials_path().dimmed());
        }
        Err(e) => {
            spinner.finish_and_clear();
            ui::print_error("API key invalid or expired");
            println!();
            println!("  Error: {}", e);
            println!();
            println!("  Run {} to re-authenticate.", "hs auth login".cyan());
        }
    }

    Ok(())
}
