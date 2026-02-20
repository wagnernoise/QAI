pub mod tui;

pub use tui::{render_to_buffer, save_api_token, load_api_token, strip_model_tags, App, ChatFocus, Provider, Screen};

use anyhow::{bail, Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn read_prompt(prompt: &PathBuf) -> Result<String> {
    fs::read_to_string(prompt)
        .with_context(|| format!("Failed to read prompt at {}", prompt.display()))
}

pub fn info(prompt: &PathBuf) -> Result<()> {
    let prompt_exists = prompt.exists();
    println!("QAI CLI");
    println!("Prompt path: {}", prompt.display());
    println!("Prompt exists: {}", prompt_exists);
    println!("README: README.md");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

pub fn show(prompt: &PathBuf) -> Result<()> {
    let content = read_prompt(prompt)?;
    print!("{}", content);
    io::stdout().flush().ok();
    Ok(())
}

pub fn copy(prompt: &PathBuf, dest: PathBuf, force: bool) -> Result<()> {
    if dest.exists() && !force {
        bail!(
            "Destination already exists. Use --force to overwrite: {}",
            dest.display()
        );
    }
    let content = read_prompt(prompt)?;
    fs::write(&dest, content)
        .with_context(|| format!("Failed to write to {}", dest.display()))?;
    println!("Copied prompt to {}", dest.display());
    Ok(())
}

pub fn validate(prompt: &PathBuf) -> Result<()> {
    let content = read_prompt(prompt)?;
    let required = [
        "## ENVIRONMENT",
        "### PRIMARY OBJECTIVE",
        "### MODE SELECTION PRIMER",
    ];

    let mut missing = Vec::new();
    for marker in required {
        if !content.contains(marker) {
            missing.push(marker);
        }
    }

    if missing.is_empty() {
        println!("Prompt validation passed.");
        Ok(())
    } else {
        bail!(
            "Prompt validation failed. Missing sections: {}",
            missing.join(", ")
        )
    }
}

pub fn tools() -> Result<()> {
    println!("Expected tool categories:");
    println!("- File operations");
    println!("- Terminal / bash");
    println!("- Web search");
    Ok(())
}
