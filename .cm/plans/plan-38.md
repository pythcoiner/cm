# Plan: Add Shorthand Model Selection (sonnet/opus)

## Summary

Add `--model` flag that accepts `sonnet` or `opus` and passes it directly to claude CLI. No hardcoded model version strings - let claude resolve to the latest.

## Claude CLI Reference

From `claude --help`:
```
  --model <model>                                   Model for the current session. Provide an alias for the latest model (e.g. 'sonnet' or 'opus') or a model's full name (e.g.
                                                    'claude-sonnet-4-5-20250929').
```

Claude CLI handles alias resolution internally, so we just pass `sonnet` or `opus`.

## Desired Behavior

```bash
cm --model sonnet   # Passes --model sonnet to claude CLI
cm --model opus     # Passes --model opus to claude CLI
```

## Files to Modify

### 1. `src/cli/mod.rs`

Use clap's `value_enum` to restrict to valid values:

```rust
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ModelChoice {
    Sonnet,
    Opus,
}

// In Cli struct:
#[arg(long, value_enum)]
pub model: Option<ModelChoice>,
```

In `build_manager_config()`:

```rust
if let Some(model_choice) = cli.model {
    let model_str = match model_choice {
        ModelChoice::Sonnet => "sonnet",
        ModelChoice::Opus => "opus",
    };
    config = config.model(model_str.to_string());
}
```

### 2. `src/manager/mod.rs`

Update default model (line ~129):

```rust
// Change from: "claude-sonnet-4-5-20250929"
// To:
model: String::from("sonnet"),
```

## Implementation Steps

1. Add `ModelChoice` enum with `#[derive(ValueEnum)]`
2. Change `model: Option<String>` to `model: Option<ModelChoice>` in Cli struct
3. Map enum to simple strings: `Sonnet => "sonnet"`, `Opus => "opus"`
4. Update default model in ManagerConfig to `"sonnet"`
5. Config file can still use `model = "sonnet"` or `model = "opus"`
6. Run `cargo build`, `cargo clippy`, `cargo test`
