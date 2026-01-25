# Plan: Add Range-Based Phase Selection

## Summary

Enhance the `p` command to support range syntax like `p 3-6` which expands to phases 3, 4, 5, 6. Mixed usage like `p 1 3-5 8` should also work.

## File to Modify

**`src/manager/mod.rs`** - lines 599-610 in `prompt_task_selection()`

## Current Code

```rust
_ if input.starts_with("p ") || input.starts_with("phase ") => {
    let nums_part = input.strip_prefix("p ").or_else(|| input.strip_prefix("phase ")).unwrap();
    let phase_ids: Vec<String> = nums_part
        .split_whitespace()
        .map(|n| format!("phase-{}", n))
        .collect();
    // ...
}
```

## New Logic

Replace the simple `.map()` with a `.flat_map()` that:
1. For each token, check if it contains `-`
2. If yes: parse as range `start-end`, generate all phase IDs from start to end (inclusive)
3. If no: treat as single phase number

## Implementation

```rust
let phase_ids: Vec<String> = nums_part
    .split_whitespace()
    .flat_map(|token| {
        if let Some((start, end)) = token.split_once('-') {
            // Range: "3-6" -> ["phase-3", "phase-4", "phase-5", "phase-6"]
            let start: u32 = start.parse().unwrap_or(0);
            let end: u32 = end.parse().unwrap_or(0);
            (start..=end).map(|n| format!("phase-{}", n)).collect::<Vec<_>>()
        } else {
            // Single: "3" -> ["phase-3"]
            vec![format!("phase-{}", token)]
        }
    })
    .collect();
```

## Edge Cases

- `p 3-3` → `["phase-3"]` (single phase)
- `p 5-3` → empty (start > end, could warn user)
- `p 1 3-5 8` → `["phase-1", "phase-3", "phase-4", "phase-5", "phase-8"]`
- `p abc-def` → invalid parse, falls back to 0-0

## Optional: Update Prompt Text

Also update the prompt text (around line 568) to show the range syntax:

```
[s]ingle / [a]ll / [p]hase <#...> / [q]uit:
```

Could become:

```
[s]ingle / [a]ll / [p]hase <# or #-#> / [q]uit:
```

## Tests

Add test cases for range parsing in the existing test module.
