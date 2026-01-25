# Plan: Add .cm/logs/ to .gitignore during cm init

## Summary
When running `cm init`, ensure that `.cm/logs/` is added to the repository's root `.gitignore` file.

## File to Modify

- `src/cli/init.rs`

## Implementation Steps

### 1. Add helper function `ensure_gitignore_entry()`

Add a new function after `cleanup_legacy_skills()`:

```rust
/// Ensure `.cm/logs/` is in the root .gitignore file.
///
/// Creates the .gitignore file if it doesn't exist.
/// Appends the entry if not already present.
fn ensure_gitignore_entry(base_dir: &Path) -> Result<(), CliError> {
    let gitignore_path = base_dir.join(".gitignore");
    let entry = ".cm/logs/";

    // Read existing content if file exists
    let content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)?
    } else {
        String::new()
    };

    // Check if entry already exists (as a complete line)
    let already_present = content
        .lines()
        .any(|line| line.trim() == entry);

    if already_present {
        return Ok(());
    }

    // Append the entry
    let mut new_content = content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(entry);
    new_content.push('\n');

    fs::write(&gitignore_path, new_content)?;
    info!("Added '{}' to .gitignore", entry);
    println!("Added '{}' to .gitignore", entry);

    Ok(())
}
```

### 2. Call from `execute_init_in_dir()`

Add call after writing template files, before `cleanup_legacy_skills()`:

```rust
    // ... (after writing template files)

    // Ensure .cm/logs/ is in .gitignore
    ensure_gitignore_entry(base_dir)?;

    // Clean up legacy skills directory if it exists
    cleanup_legacy_skills(base_dir)?;
```

### 3. Add tests

Add tests at the end of the test module:

```rust
#[test]
fn test_ensure_gitignore_creates_file() {
    let tmp = TempDir::new().unwrap();
    execute_init_in_dir(tmp.path(), false).unwrap();

    let gitignore = tmp.path().join(".gitignore");
    assert!(gitignore.exists());
    let content = fs::read_to_string(&gitignore).unwrap();
    assert!(content.contains(".cm/logs/"));
}

#[test]
fn test_ensure_gitignore_appends_to_existing() {
    let tmp = TempDir::new().unwrap();
    let gitignore = tmp.path().join(".gitignore");
    fs::write(&gitignore, "node_modules/\n").unwrap();

    execute_init_in_dir(tmp.path(), false).unwrap();

    let content = fs::read_to_string(&gitignore).unwrap();
    assert!(content.contains("node_modules/"));
    assert!(content.contains(".cm/logs/"));
}

#[test]
fn test_ensure_gitignore_idempotent() {
    let tmp = TempDir::new().unwrap();
    execute_init_in_dir(tmp.path(), false).unwrap();
    execute_init_in_dir(tmp.path(), false).unwrap();

    let gitignore = tmp.path().join(".gitignore");
    let content = fs::read_to_string(&gitignore).unwrap();
    // Should only appear once
    assert_eq!(content.matches(".cm/logs/").count(), 1);
}
```

## Testing

1. Run `cargo test` to verify new tests pass
2. Run `cargo build` and `cargo clippy` to ensure no errors
3. Manual test: run `cm init` in a fresh directory, verify `.gitignore` is created with `.cm/logs/`
