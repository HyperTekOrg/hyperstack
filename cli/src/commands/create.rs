use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::fs;
use std::path::Path;

use crate::templates::{
    customize_project, detect_package_manager, dev_command, install_command, Template,
    TemplateManager,
};
use crate::ui;

pub fn create(
    name: Option<String>,
    template: Option<String>,
    offline: bool,
    force_refresh: bool,
) -> Result<()> {
    let theme = ColorfulTheme::default();

    let project_name = match name {
        Some(n) => n,
        None => Input::with_theme(&theme)
            .with_prompt("Project name")
            .default("my-hyperstack-app".to_string())
            .interact_text()
            .context("Failed to read project name")?,
    };

    let selected_template = match template {
        Some(t) => Template::from_str(&t).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown template: {}. Available: react-pumpfun, react-ore",
                t
            )
        })?,
        None => {
            let items: Vec<String> = Template::ALL
                .iter()
                .map(|t| format!("{} - {}", t.display_name(), t.description()))
                .collect();

            let selection = Select::with_theme(&theme)
                .with_prompt("Select a template")
                .items(&items)
                .default(0)
                .interact()
                .context("Failed to select template")?;

            Template::ALL[selection]
        }
    };

    let project_dir = Path::new(&project_name);

    if project_dir.exists() {
        anyhow::bail!(
            "Directory '{}' already exists. Choose a different name or remove it first.",
            project_name
        );
    }

    let manager = TemplateManager::new()?;

    if force_refresh {
        ui::print_step("Clearing template cache...");
        manager.clear_cache()?;
    }

    if !manager.is_cached() {
        if offline {
            anyhow::bail!(
                "Templates not cached and --offline specified. Run without --offline first."
            );
        }

        ui::print_step("Downloading templates...");
        manager.fetch_templates()?;
        println!("  {} Templates cached", ui::symbols::SUCCESS.green());
    }

    ui::print_step(&format!(
        "Creating {} from {}...",
        project_name.bold(),
        selected_template.display_name().cyan()
    ));

    fs::create_dir_all(project_dir)
        .with_context(|| format!("Failed to create directory: {}", project_name))?;

    manager.copy_template(selected_template, project_dir)?;
    customize_project(project_dir, &project_name)?;

    println!();
    println!(
        "{} Created {}",
        ui::symbols::SUCCESS.green().bold(),
        project_name.bold()
    );

    let pm = detect_package_manager();
    println!();
    println!("Next steps:");
    println!("  {} {}", "cd".dimmed(), project_name);
    println!("  {}", install_command(pm).dimmed());
    println!("  {}", dev_command(pm).dimmed());
    println!();

    Ok(())
}
