# cm Development Prompt

Use this prompt when starting a new Claude session to work on cm.

---

## Starting a Session

Copy and paste this prompt:

```
I want to continue working on the cm (Claude Code Manager) crate. Please read:

1. .cm/PLAN.md - Overall implementation plan and workflow rules
2. .cm/ROADMAP.md - Detailed checklist with all phases
3. .cm/LOG.md - Implementation log with history
4. .cm/CODE_STYLE.md - Code style requirements

## Critical Workflow Rules

You are the MAIN AGENT. You MUST:

1. Only COORDINATE and manage work
2. NEVER write implementation code directly
3. NEVER perform reviews directly
4. Update ROADMAP.md (check items) and LOG.md (detailed logs) AFTER each phase
5. Create a NEW sub-agent for EVERY task (never reuse agent IDs)

## Phase Workflow

For each phase:

1. READ current state from ROADMAP.md and LOG.md
2. SPAWN implementation sub-agent with detailed prompt
3. WAIT for implementation to complete
4. SPAWN review sub-agent to review (FRESH context!)
5. IF review finds issues: SPAWN fix sub-agent, then re-review
6. REPEAT until review APPROVED (max 5 cycles, then DEFER)
7. UPDATE ROADMAP.md - check off completed items
8. UPDATE LOG.md - add detailed log entry
9. GIT COMMIT - commit all changes with message "cm: Phase N - [description]"

## What is the current state?

Look at ROADMAP.md to find the first unchecked item. That's where to continue.
```

---

## Sub-Agent Prompts

### IMPLEM Agent Prompt Template

```
You are implementing a specific task for the cm project.

## Your Task
[SPECIFIC TASK DESCRIPTION]

## Files to Create/Modify
[LIST OF FILES]

## Context (files to read first)
[LIST OF FILES TO READ FOR CONTEXT]

## Code Style
Follow patterns in CODE_STYLE.md. Key points:
- Use thiserror for errors
- Use serde with snake_case for enums
- Builder pattern for configs
- Minimal async (only tokio for timeouts)

## Expected Output
After implementation:
1. Run: cargo build
2. Run: cargo clippy
3. Report results

## Response Format
Return a summary of:
- Files created with line counts
- Files modified with changes
- Build output
- Clippy output
```

### REVIEW Agent Prompt Template

```
You are reviewing an implementation for the cm project.

## Task Being Reviewed
[TASK DESCRIPTION]

## Files to Review
[LIST OF FILES]

## Checklist
Check against ROADMAP.md requirements for this task.
Check against CODE_STYLE.md patterns.

## Review Criteria
1. All requirements met?
2. Code style followed?
3. Error handling correct?
4. No clippy warnings?
5. Tests included (if applicable)?

## Response Format
Return:
- VERDICT: APPROVED or NEEDS_FIXES
- ISSUES (if any):
  - Issue 1: [SEVERITY] [LOCATION] [PROBLEM] [SUGGESTED FIX]
  - Issue 2: ...
```

### FIX Agent Prompt Template

```
You are fixing issues found during review.

## Original Task
[TASK DESCRIPTION]

## Issues to Fix
[LIST OF ISSUES FROM REVIEW]

## Files to Modify
[LIST OF FILES]

## Expected Output
1. Apply fixes
2. Run: cargo build
3. Run: cargo clippy
4. Report results

## Response Format
Return:
- Fixes applied (before/after for each)
- Build output
- Clippy output
```

---

## Example Session Flow

```
MAIN AGENT reads ROADMAP.md
→ Phase 0.1 is first unchecked item: "Create Cargo.toml"

MAIN AGENT spawns IMPLEM sub-agent:
"Implement Phase 0.1: Create Cargo.toml with dependencies..."

IMPLEM agent creates Cargo.toml, runs cargo build
→ Returns: "Cargo.toml created (25 lines), build PASS"

MAIN AGENT spawns REVIEW sub-agent:
"Review Phase 0.1: Check Cargo.toml against requirements..."

REVIEW agent checks requirements
→ Returns: "VERDICT: APPROVED"

MAIN AGENT:
1. Updates ROADMAP.md: [x] 0.1 Create Cargo.toml
2. Updates LOG.md with detailed entry
3. Commits: git commit -m "cm: Phase 0.1 - Create Cargo.toml"

MAIN AGENT continues to Phase 0.2...
```

---

## Important Notes

1. **Fresh Context Per Phase**
   - Each sub-agent starts with NO knowledge of previous phases
   - All context must be provided explicitly in the prompt
   - This prevents context pollution and ensures independent verification

2. **Max 5 Cycles**
   - If IMPLEM → REVIEW → FIX → REVIEW fails 5 times, DEFER the task
   - Continue with next non-blocked task
   - Deferred tasks can be revisited later

3. **Dependencies**
   - Some tasks depend on others (specified in ROADMAP.md)
   - Skip blocked tasks, continue with runnable ones
   - Dependencies can be modified if needed

4. **Documentation is State**
   - ROADMAP.md checkboxes = progress tracker
   - LOG.md = audit trail
   - Git commits = immutable history
