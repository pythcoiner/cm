//! Claude Code Manager (cm) - CLI entry point.
//!
//! This is the main entry point for the cm binary. It parses command-line
//! arguments and dispatches to the appropriate functionality.

use std::path::PathBuf;

use clap::Parser;

/// Claude Code Manager - Automated agent coordination for software development.
///
/// Orchestrates Claude agents to execute multi-phase development tasks
/// defined in a tasks.json file.
#[derive(Debug, Parser)]
#[command(name = "cm")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Resume from interrupted state.
    #[arg(long)]
    r#continue: bool,

    /// Execute one task, then pause.
    #[arg(long)]
    step: bool,

    /// Show progress without executing.
    #[arg(long)]
    status: bool,

    /// Validate tasks.json schema without executing.
    #[arg(long)]
    validate: bool,

    /// Enable verbose output.
    #[arg(short, long)]
    verbose: bool,

    /// Path to config file.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Path to tasks.json state file.
    #[arg(long, value_name = "FILE", default_value = ".cm/tasks.json")]
    state: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    // Initialize logger based on verbosity
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("Claude Code Manager starting...");
    log::debug!("CLI arguments: {:?}", cli);

    // Dispatch based on flags
    if cli.status {
        println!("Status mode: Would show progress from {:?}", cli.state);
        println!("(Not yet implemented)");
    } else if cli.validate {
        println!("Validate mode: Would validate {:?}", cli.state);
        println!("(Not yet implemented)");
    } else if cli.step {
        println!("Step mode: Would execute one task from {:?}", cli.state);
        println!("(Not yet implemented)");
    } else if cli.r#continue {
        println!("Continue mode: Would resume from {:?}", cli.state);
        println!("(Not yet implemented)");
    } else {
        println!("Run mode: Would execute all tasks from {:?}", cli.state);
        println!("(Not yet implemented)");
    }

    if let Some(config) = &cli.config {
        log::debug!("Using config file: {:?}", config);
    }

    log::info!("Claude Code Manager finished.");
}
