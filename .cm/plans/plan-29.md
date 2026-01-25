Add a --reset <PHASE_ID> CLI flag that resets a phase to pending status:

1. Add CLI argument to `Cli` struct in `src/cli/mod.rs` (with other flags):
```rust
/// Reset a phase to pending status and clear execution state.
#[arg(long, value_name = "PHASE_ID")]
pub reset: Option<String>,
```

2. Add flag dispatch in `run()` function (around line 204-222):
```rust
} else if cli.reset.is_some() {
    execute_reset(&cli)
```

3. Implement `execute_reset()` function:
```rust
fn execute_reset(cli: &Cli) -> Result<(), CliError> {
    info!("Reset mode: resetting phase {:?}", cli.reset);

    let phase_id = cli.reset.as_ref().unwrap();
    let mut state = load_state(&cli.state)?;

    // Find and reset the phase
    let phase = state.get_phase_mut(phase_id)?;

    // Reset phase fields
    phase.status = PhaseStatus::Pending;
    phase.review_cycles_completed = 0;
    phase.baseline_commit = None;
    phase.implem_completed_at = None;

    // Reset all tasks in the phase
    for task in &mut phase.tasks {
        task.status = TaskStatus::Pending;
        task.attempts.clear();
        task.implem_completed_at = None;
        task.baseline_commit = None;
        task.review_cycles_completed = 0;
    }

    // Save modified state
    save_state(&state, &cli.state)?;

    println!("Phase '{}' reset to pending state", phase_id);
    println!("  - Phase status: pending");
    println!("  - {} task(s) reset", phase.tasks.len());

    Ok(())
}
```

4. Update `CLAUDE.md` CLI modes section to add:
```
cm --reset <ID>     # Reset phase to pending
```

5. Ensure `cargo build` and `cargo clippy` pass
6. Ensure `cargo test` passes
