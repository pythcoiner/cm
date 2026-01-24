# Claude Code Manager (cm)

Automated agent coordination for software development.

`cm` orchestrates Claude AI agents to execute multi-phase development tasks
autonomously. Agents work in isolation - each receives only task-specific context,
ensuring independent verification and preventing context pollution.

## Features

- **Multi-phase task orchestration** - Define phases with dependent tasks
- **Agent context isolation** - No direct agent-to-agent communication
- **Build verification** - Automatic cargo build + clippy checks
- **Crash recovery** - Checkpointing with state restoration
- **Real-time TUI** - Terminal UI for monitoring with `--tui`
- **Config file support** - TOML configuration via `.cm/config.toml`
- **Signal handling** - Graceful Ctrl+C shutdown with state preservation
- **Audit log** - Append-only LOG.md tracks all actions
- **Deterministic generation** - JSON is source of truth, MD files regenerated

## Installation

### Prerequisites

- Rust toolchain (1.70+)
- [Claude CLI](https://github.com/anthropics/claude-code) installed and configured

### Build from source

```bash
git clone https://github.com/your-repo/cm.git
cd cm
cargo build --release
```

### Install to PATH (optional)

```bash
cargo install --path .
```

## Quick Start

### Install skills into your project

```bash
cm init              # Creates .claude/skills/{cm,feat,fix}.md
```

### Use the /cm skill in Claude Code

```bash
claude
> /cm               # Interactive wizard to set up a new cm project
```

This creates:
- `.cm/tasks.json` - Task definitions & state
- `.cm/PLAN.md` - High-level project plan
- `.cm/ROADMAP.md` - Detailed checklist
- `.cm/LOG.md` - Execution log

### Run cm

```bash
cm                  # Run all tasks until completion
```

## Usage

```bash
# Execution
cm                  # Run all tasks until completion
cm --tui            # Run with terminal UI
cm --continue       # Resume from interrupted state
cm --step           # Execute one task, then pause
cm --dry-run        # Preview without executing

# Status
cm --status         # Show progress summary
cm --validate       # Validate tasks.json schema
cm --regenerate     # Regenerate MD files from JSON

# Options
cm -v, --verbose    # Enable debug logging
cm --config FILE    # Custom config file (default: .cm/config.toml)
cm --state FILE     # Custom state file (default: .cm/tasks.json)
cm --model MODEL    # Override model
cm --timeout SECS   # Override agent timeout
```

## Skills

`cm init` installs three Claude Code skills:

| Skill | Usage | Description |
|-------|-------|-------------|
| `/cm` | `/cm` | Interactive wizard to set up a new cm project |
| `/feat` | `/feat` | Add a new feature to an existing cm project |
| `/fix` | `/fix` | Add a bug fix task to an existing cm project |

## Configuration

Create `.cm/config.toml` for persistent settings:

```toml
model = "claude-sonnet-4-5-20250929"
timeout_secs = 300
max_cycles = 5
log_path = ".cm/LOG.md"
```

See [CONFIG.md](CONFIG.md) for all options.

**Precedence:** CLI flags > config file > defaults

## Project Structure

```
your-project/
├── .cm/
│   ├── tasks.json      # Task definitions & state (source of truth)
│   ├── roadmap.json    # Roadmap state (optional)
│   ├── config.toml     # Configuration (optional)
│   ├── PLAN.md         # High-level plan
│   ├── ROADMAP.md      # Detailed checklist (generated)
│   ├── LOG.md          # Audit trail (generated)
│   └── checkpoints/    # Recovery snapshots (auto-generated)
├── .claude/
│   └── skills/         # Installed skills (from cm init)
├── src/
└── Cargo.toml
```

## TUI Controls

When running with `--tui`:

| Key | Action |
|-----|--------|
| `p` | Pause/resume after current task |
| `i` | Interrupt immediately and save state |
| `q` | Quit (same as interrupt) |
| `↑/↓` | Scroll output view |
| `PageUp/PageDown` | Scroll by page |

## How It Works

1. **Task Selection** - Manager picks the next runnable task (respects dependencies)
2. **Implementation** - Spawns IMPLEM agent with task-only context
3. **Verification** - Runs `cargo build` and `cargo clippy`
4. **Review** - Spawns REVIEW agent with fresh context
5. **Iteration** - If review finds issues, creates FIX task and re-reviews
6. **Completion** - After approval, marks task complete

Tasks are retried up to 5 times before being deferred. All actions are logged.

## JSON as Source of Truth

`cm` uses JSON files as the source of truth:
- `tasks.json` - Task state and log records
- `roadmap.json` - Roadmap structure

Markdown files (LOG.md, ROADMAP.md) are **generated** from JSON:
```bash
cm --regenerate     # Regenerate all MD files from JSON
```

## License

MIT
