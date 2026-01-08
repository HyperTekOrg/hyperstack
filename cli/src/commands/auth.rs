use anyhow::Result;
use colored::Colorize;

use crate::api_client::ApiClient;

pub fn register() -> Result<()> {
    println!("{}", "Register for Hyperstack".bold());
    println!();

    print!("Username: ");
    use std::io::{self, Write};
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
    println!("Creating account...");

    let client = ApiClient::new()?;
    let response = client.register(username, &password)?;

    println!();
    println!("{}", "✓ Account created successfully!".green().bold());
    println!("Username: {}", response.user.username);
    
    if let Some(api_key) = &response.api_key {
        ApiClient::save_api_key(api_key)?;
        println!("{}", "✓ API key saved".green());
        println!();
        println!("You are now logged in and ready to use Hyperstack!");
    } else {
        println!();
        println!("{}", response.message.yellow());
        println!("Run 'hyperstack auth login' to authenticate.");
    }

    Ok(())
}

pub fn login() -> Result<()> {
    println!("{}", "Login to Hyperstack".bold());
    println!();

    print!("Username: ");
    use std::io::{self, Write};
    io::stdout().flush()?;
    
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim();

    let password = rpassword::prompt_password("Password: ")?;

    println!();
    println!("Logging in...");

    let client = ApiClient::new()?;
    let response = client.login(username, &password)?;

    println!();
    println!("{}", "✓ Login successful!".green().bold());
    println!("Username: {}", response.user.username);
    
    if let Some(api_key) = &response.api_key {
        ApiClient::save_api_key(api_key)?;
        println!("{}", "✓ API key saved".green());
    }

    println!();
    println!("{}", response.message);

    Ok(())
}

pub fn logout() -> Result<()> {
    println!("Logging out...");
    
    ApiClient::delete_api_key()?;
    
    println!("{}", "✓ Logged out successfully".green().bold());
    println!("Your credentials have been removed from this device.");

    Ok(())
}

pub fn status() -> Result<()> {
    match ApiClient::load_api_key() {
        Ok(_) => {
            println!("{}", "OK Authenticated".green().bold());
            println!();
            println!("You are logged in and ready to use Hyperstack.");
            println!();
            println!("API key location: {}", 
                ApiClient::new()
                    .ok()
                    .and_then(|_| dirs::home_dir())
                    .map(|home| home.join(".hyperstack").join("credentials.toml").display().to_string())
                    .unwrap_or_else(|| "~/.hyperstack/credentials.toml".to_string())
            );
        }
        Err(_) => {
            println!("{}", "ERR Not authenticated".red().bold());
            println!();
            println!("Run 'hyperstack auth login' to authenticate.");
        }
    }

    Ok(())
}

/// Verify the current authentication by making an API call
pub fn whoami() -> Result<()> {
    let api_key = match ApiClient::load_api_key() {
        Ok(key) => key,
        Err(_) => {
            println!("{}", "ERR Not authenticated".red().bold());
            println!();
            println!("Run 'hyperstack auth login' to authenticate.");
            return Ok(());
        }
    };

    println!("{} Verifying authentication...", "->".blue().bold());

    let client = ApiClient::new()?;
    
    // Try to list specs as a way to verify the API key is valid
    match client.list_specs() {
        Ok(specs) => {
            println!("{}", "OK Authenticated".green().bold());
            println!();
            println!("  API key: {}...{}", &api_key[..8], &api_key[api_key.len()-4..]);
            println!("  Specs: {}", specs.len());
            println!();
            println!("API key location: {}", 
                dirs::home_dir()
                    .map(|home| home.join(".hyperstack").join("credentials.toml").display().to_string())
                    .unwrap_or_else(|| "~/.hyperstack/credentials.toml".to_string())
            );
        }
        Err(e) => {
            println!("{}", "ERR API key invalid or expired".red().bold());
            println!();
            println!("Error: {}", e);
            println!();
            println!("Run 'hyperstack auth login' to re-authenticate.");
        }
    }

    Ok(())
}

