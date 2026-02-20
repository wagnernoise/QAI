use qai_cli::{copy, info, show, tools, validate};

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "qai-cli", version, about = "Manage the QAI agent prompt")]
struct Cli {
    /// Path to the qa-agent system prompt
    #[arg(long, default_value = "qa-agent-system-prompt.md")]
    prompt: PathBuf,
    /// Skip the TUI and run a subcommand directly
    #[arg(long)]
    no_tui: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show basic information about the agent prompt
    Info,
    /// Print the system prompt to stdout
    Show,
    /// Copy the system prompt to a destination file
    Copy {
        /// Destination path for the prompt file
        dest: PathBuf,
        /// Overwrite the destination if it already exists
        #[arg(long)]
        force: bool,
    },
    /// Validate that the prompt contains expected sections
    Validate,
    /// Print the expected tool categories for QA-Bot
    Tools,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // If a subcommand is given or --no-tui is set, run in CLI mode
    if cli.no_tui || cli.command.is_some() {
        match cli.command {
            Some(Commands::Info) => info(&cli.prompt),
            Some(Commands::Show) => show(&cli.prompt),
            Some(Commands::Copy { dest, force }) => copy(&cli.prompt, dest, force),
            Some(Commands::Validate) => validate(&cli.prompt),
            Some(Commands::Tools) => tools(),
            None => {
                eprintln!("No subcommand given. Run without --no-tui to launch the TUI.");
                Ok(())
            }
        }
    } else {
        qai_cli::tui::run(cli.prompt).await
    }
}
