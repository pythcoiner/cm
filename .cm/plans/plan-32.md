# Plan: Add Phase Selection by Number

## Summary
Add a `[p]hase <#...>` option to the interactive prompt, allowing users to run specific phases by number (space-separated).

**Input format**: `p 3 5 7` runs phases 3, 5, and 7
**Error handling**: Warn about invalid/completed phases but continue with valid ones

## Files to Modify

- `/home/user/cm/src/manager/mod.rs` (all changes in this file)

## Implementation Steps

### 1. Update TaskSelection enum (line 37-46)

Add a new variant to hold specific phase IDs:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]  // Remove Copy (Vec not Copy)
pub enum TaskSelection {
    Single,
    All,
    Phases(Vec<String>),  // NEW: specific phase IDs like ["phase-3", "phase-5"]
    Quit,
}
```

### 2. Update prompt text (line 580)

Change from:
```rust
print!("\n[s]ingle / [a]ll / [q]uit: ");
```

To:
```rust
print!("\n[s]ingle / [a]ll / [p]hase <#...> / [q]uit: ");
```

### 3. Update input parsing (lines 592-599)

Add parsing for `p <numbers>` input. The logic:
- If input starts with `p ` or `phase `, extract the rest
- Split by whitespace to get individual phase numbers
- Convert each number N to `"phase-N"`
- Return `TaskSelection::Phases(phase_ids)`

```rust
match line.trim().to_lowercase().as_str() {
    "s" | "single" => Ok(TaskSelection::Single),
    "a" | "all" => Ok(TaskSelection::All),
    "q" | "quit" | "" => Ok(TaskSelection::Quit),
    input if input.starts_with("p ") || input.starts_with("phase ") => {
        let nums_part = input.strip_prefix("p ").or_else(|| input.strip_prefix("phase ")).unwrap();
        let phase_ids: Vec<String> = nums_part
            .split_whitespace()
            .map(|n| format!("phase-{}", n))
            .collect();
        if phase_ids.is_empty() {
            println!("No phase numbers provided.");
            self.prompt_task_selection()
        } else {
            Ok(TaskSelection::Phases(phase_ids))
        }
    }
    _ => {
        println!("Invalid selection. Use 's', 'a', 'p <#...>', or 'q'.");
        self.prompt_task_selection()
    }
}
```

### 4. Update run_interactive() (lines 611-633)

Add handling for the new `Phases` variant:

```rust
TaskSelection::Phases(phase_ids) => {
    self.run_specific_phases(&phase_ids)?;
    // Loop back to prompt for another selection
}
```

### 5. Add new method: run_specific_phases()

Add a new method to execute specific phases (after `run_interactive()`):

```rust
/// Run specific phases by ID.
///
/// Skips phases that don't exist or are already completed, with warnings.
fn run_specific_phases(&mut self, phase_ids: &[String]) -> Result<(), ManagerError> {
    for phase_id in phase_ids {
        // Reload state to check current status
        let state = load_state(&self.config.state_path)?;

        // Find the phase
        let phase = state.phases.iter().find(|p| p.id == *phase_id);

        match phase {
            None => {
                println!("Warning: Phase '{}' not found, skipping.", phase_id);
                continue;
            }
            Some(p) if p.status == PhaseStatus::Completed => {
                println!("Warning: Phase '{}' already completed, skipping.", phase_id);
                continue;
            }
            Some(_) => {
                // Execute the phase
                match self.execute_phase(phase_id) {
                    Ok(()) => {
                        self.update_state()?;
                    }
                    Err(e) => {
                        println!("Phase '{}' failed: {}", phase_id, e);
                        // Continue with next phase
                    }
                }
            }
        }
    }
    Ok(())
}
```

## Testing

1. Run `cm --daemon` and verify:
   - `p 31` runs phase-31
   - `p 31 32` runs phases 31 and 32
   - `p 999` shows warning "Phase 'phase-999' not found"
   - `p 1` shows warning "Phase 'phase-1' already completed" (assuming it is)

2. Ensure `cargo build` and `cargo clippy` pass
