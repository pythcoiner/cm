Add `.cm/*.log` to root `.gitignore` during `cm init`:

1. Add function to `src/cli/init.rs`:
```rust
/// Ensure .cm/*.log is in the root .gitignore file.
fn ensure_gitignore_entry(base_dir: &Path) -> Result<(), CliError> {
    let gitignore_path = base_dir.join(".gitignore");
    let entry = ".cm/*.log";

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)?;
        if content.lines().any(|line| line.trim() == entry) {
            return Ok(());
        }
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&gitignore_path)?;
        use std::io::Write;
        writeln!(file, "\n# cm runtime logs\n{}", entry)?;
    } else {
        fs::write(&gitignore_path, format!("# cm runtime logs\n{}\n", entry))?;
    }

    println!("Added {} to .gitignore", entry);
    Ok(())
}
```

2. Call from `execute_init_in_dir()` after creating directories:
```rust
ensure_gitignore_entry(base_dir)?;
```

3. Add tests:
```rust
#[test]
fn test_execute_init_adds_gitignore_entry() {
    let tmp = TempDir::new().unwrap();
    execute_init_in_dir(tmp.path(), false).unwrap();
    let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(content.contains(".cm/*.log"));
}

#[test]
fn test_execute_init_appends_to_existing_gitignore() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".gitignore"), "node_modules/\n").unwrap();
    execute_init_in_dir(tmp.path(), false).unwrap();
    let content = fs::read_to_string(tmp.path().join(".gitignore")).unwrap();
    assert!(content.contains("node_modules/"));
    assert!(content.contains(".cm/*.log"));
}
```

4. Ensure `cargo build` and `cargo clippy` pass
5. Ensure `cargo test` passes
