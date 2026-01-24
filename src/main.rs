//! Claude Code Manager (cm) - CLI entry point.
//!
//! This is the main entry point for the cm binary. It delegates all work
//! to the cli module.

use cm::cli;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
