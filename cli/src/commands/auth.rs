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

pub fn register() -> Result<()> {
    println!("{}", "Register for Hyperstack".bold());
    println!();

    print!("Username: ");
    io::stdout().flush()?;

    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim();

    let password = rpassword::prompt_password("Password: ")?;
    let confirm_password = rpassword::prompt_password("Confirm password: ")?;

    if password != confirm_password {
        anyhow::bail!("Passwords do not match");
    }

    if password.len() < 8 {
        anyhow::bail!("Password must be at least 8 characters long");
    }

    println!();
    let spinner = ui::create_spinner("Creating account...");

    let client = ApiClient::new()?;
    let response = client.register(username, &password)?;

    spinner.finish_and_clear();
    ui::print_success("Account created successfully!");
    println!("  Username: {}", response.user.username);

    if let Some(api_key) = &response.api_key {
        ApiClient::save_api_key(api_key)?;
        ui::print_success("API key saved");
        println!();
        println!("You are now logged in and ready to use Hyperstack!");
    } else {
        println!();
        ui::print_warning(&response.message);
        println!("Run 'hs auth login' to authenticate.");
    }

    Ok(())
}

pub fn login() -> Result<()> {
    println!("{}", "Login to Hyperstack".bold());
    println!();

    print!("Username: ");
    io::stdout().flush()?;

    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim();

    let password = rpassword::prompt_password("Password: ")?;

    println!();
    let spinner = ui::create_spinner("Logging in...");

    let client = ApiClient::new()?;
    let response = client.login(username, &password)?;

    spinner.finish_and_clear();
    ui::print_success("Login successful!");
    println!("  Username: {}", response.user.username);

    if let Some(api_key) = &response.api_key {
        ApiClient::save_api_key(api_key)?;
        ui::print_success("API key saved");
    }

    println!();
    println!("{}", response.message);

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
