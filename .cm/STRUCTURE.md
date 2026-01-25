# Project Structure

## Directories

```
cm/
├── src/
│   ├── main.rs          # Entry point, delegates to cli::run
│   ├── lib.rs           # Module exports
│   ├── cli/
│   │   ├── mod.rs       # Clap-based argument parsing
│   │   └── init.rs      # cm init subcommand
│   ├── command.rs       # Embedded .md files for /cm, /feat, /fix, /end commands
│   ├── state/
│   │   ├── mod.rs       # State loading/saving
│   │   ├── tasks.rs     # Task/phase types
│   │   ├── log_record.rs # LogRecord types
│   │   ├── roadmap.rs   # RoadmapState types
│   │   └── validate.rs  # JSON schema validation
│   ├── agent/
│   │   ├── mod.rs       # AgentSpawner, AgentHandle
│   │   ├── prompt.rs    # Prompt building with context isolation
│   │   └── response.rs  # JSON response parsing
│   ├── build/
│   │   ├── mod.rs       # BuildVerifier
│   │   ├── cargo.rs     # CargoRunner for build/clippy/test
│   │   └── git.rs       # GitRunner for status/add/commit
│   ├── manager/
│   │   ├── mod.rs       # Main orchestration loop
│   │   ├── state.rs     # ManagerState enum
│   │   └── recovery.rs  # Crash recovery, checkpointing
│   ├── log/
│   │   ├── mod.rs       # Per-phase detailed logging
│   │   └── file_logger.rs # Persistent cm.log with rotation
│   ├── generate/
│   │   ├── mod.rs       # Regenerate functions
│   │   ├── tasks_md.rs  # Generate TASKS.md from JSON
│   │   └── roadmap_md.rs # Generate ROADMAP.md from JSON
│   ├── tui/
│   │   ├── mod.rs       # App, terminal setup
│   │   ├── layout.rs    # Split view layout
│   │   └── widgets.rs   # TaskList, Stream, Controls
│   └── config/
│       └── mod.rs       # TOML configuration loading
├── tests/
│   └── integration.rs   # Integration tests
├── .cm/                  # Project state directory
│   ├── tasks.json       # Source of truth for tasks
│   ├── roadmap.json     # Source of truth for roadmap
│   ├── PLAN.md          # Project plan
│   ├── ROADMAP.md       # Generated from roadmap.json
│   ├── TASKS.md         # Generated from tasks.json
│   ├── config.toml      # Optional configuration
│   ├── cm.log           # Persistent operational log
│   ├── logs/            # Per-phase detailed logs
│   └── agents/          # Agent instruction files
├── Cargo.toml           # Rust dependencies
└── CLAUDE.md            # Claude Code instructions
```

## Entry Points

- `src/main.rs` - CLI entry point
- `cargo build --release` - Produces `target/release/cm`

## Configuration Files

- `Cargo.toml` - Rust project configuration
- `.cm/config.toml` - Optional cm configuration
- `CLAUDE.md` - Instructions for Claude Code
