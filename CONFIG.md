# Claude Code Manager - Configuration Guide

This document describes the configuration options for `cm` (Claude Code Manager).

## Configuration File

`cm` supports TOML-based configuration files. By default, it looks for `.cm/config.toml` in the current directory.

### File Location

- **Default**: `.cm/config.toml`
- **Custom**: Use `--config <path>` to specify a different config file

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `model` | String | `"claude-sonnet-4-5-20250929"` | Claude model to use for agent spawning |
| `timeout_secs` | Integer | `300` | Agent execution timeout in seconds |
| `max_cycles` | Integer | `5` | Maximum attempts per task before deferring |
| `log_path` | String | `".cm/LOG.md"` | Path to the execution log file |
| `working_dir` | String | Current directory | Working directory for build verification |

## Example Configuration

```toml
# .cm/config.toml

# Claude model to use
model = "claude-sonnet-4-5-20250929"

# Agent timeout (5 minutes)
timeout_secs = 300

# Maximum attempts before deferring a task
max_cycles = 5

# Path to execution log
log_path = ".cm/LOG.md"

# Working directory for builds (optional, defaults to current dir)
# working_dir = "/home/user/project"
```

## Precedence Rules

Configuration values are applied in the following order (highest precedence first):

1. **CLI arguments** - Command-line flags always take precedence
2. **Config file** - Values from the TOML config file
3. **Defaults** - Built-in default values

### Example

If you have this config file:

```toml
model = "claude-opus-4-5-20251101"
timeout_secs = 600
```

And run:

```bash
cm --timeout 300
```

The final configuration will be:
- `model`: `"claude-opus-4-5-20251101"` (from config file)
- `timeout_secs`: `300` (from CLI, overrides config file)
- `max_cycles`: `5` (default)

## CLI Configuration Options

All configuration options can be set via command-line arguments:

```bash
cm [OPTIONS]

Options:
    --config <FILE>       Path to config file
    --state <FILE>        Path to tasks.json (default: .cm/tasks.json)
    --model <MODEL>       Claude model to use
    --timeout <SECONDS>   Agent timeout in seconds
    --max-cycles <N>      Maximum cycles per task
    --log-path <FILE>     Path to LOG.md file
    --working-dir <DIR>   Working directory for builds
    -v, --verbose         Enable verbose output
```

## Default Values

When no config file is present and no CLI arguments are provided:

| Setting | Default Value |
|---------|---------------|
| Model | `claude-sonnet-4-5-20250929` |
| Timeout | 300 seconds (5 minutes) |
| Max Cycles | 5 |
| Log Path | `.cm/LOG.md` |
| Working Dir | Current directory |
| State Path | `.cm/tasks.json` |

## Creating a Config File

1. Create the `.cm` directory if it doesn't exist:
   ```bash
   mkdir -p .cm
   ```

2. Create the config file:
   ```bash
   cat > .cm/config.toml << 'EOF'
   model = "claude-sonnet-4-5-20250929"
   timeout_secs = 300
   max_cycles = 5
   log_path = ".cm/LOG.md"
   EOF
   ```

3. Verify the config is loaded (use `--verbose` to see config loading):
   ```bash
   cm --verbose --status
   ```

## Environment-Specific Configurations

You can maintain different config files for different environments:

```bash
# Development (longer timeouts, more retries)
cm --config .cm/config-dev.toml

# Production (stricter limits)
cm --config .cm/config-prod.toml
```

Example development config:

```toml
# .cm/config-dev.toml
model = "claude-sonnet-4-5-20250929"
timeout_secs = 600
max_cycles = 10
```

Example production config:

```toml
# .cm/config-prod.toml
model = "claude-sonnet-4-5-20250929"
timeout_secs = 180
max_cycles = 3
```
