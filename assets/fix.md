---
name: fix
description: Interactive wizard for adding bug fix tasks to an existing cm project (project)
user-invocable: true
---

# Bug Fix Wizard

This skill guides users through adding a bug fix task to an existing cm (Claude Code Manager) project. The wizard collects bug details through a conversational flow and then updates the project artifacts (tasks.json, optionally ROADMAP.md).

## Prerequisites

Before using this wizard, ensure:
- A `.cm/` directory exists with valid project files
- `tasks.json` is present
- The project has been initialized with `/cm` or `cm init`

If prerequisites are not met, inform the user and suggest running `/cm` first.

## Important: Interactive Flow

You MUST follow this wizard flow step by step. Do NOT skip steps or modify files until you have gathered all the required information and received user confirmation.

---

## Step 1: Bug Description

**Ask the user:**

> What bug needs to be fixed?
>
> Please describe:
> - **Summary**: A one-line description of the bug
> - **Observed behavior**: What is happening incorrectly?
> - **Expected behavior**: What should happen instead?
> - **Reproduction steps**: How can we reproduce this bug?

Wait for the user's response before proceeding.

---

## Step 2: Bug Classification

**Ask the user:**

> How would you classify this bug?
>
> **Severity:**
> - Critical: System crash, data loss, security vulnerability
> - High: Major feature broken, no workaround
> - Medium: Feature impaired, workaround exists
> - Low: Minor issue, cosmetic problem
>
> **Category:**
> - Logic error
> - Crash/panic
> - Performance issue
> - UI/UX problem
> - Data corruption
> - Security issue
> - Other: [specify]

Wait for the user's response before proceeding.

---

## Step 3: Investigation

**Analyze the codebase to locate the bug:**

1. Read relevant files mentioned by the user
2. Search for related code patterns
3. Identify the root cause location
4. Determine affected components

**Present your findings:**

> ## Investigation Results
>
> **Likely location:**
> - `path/to/file.rs` - [specific function or line range]
>
> **Root cause analysis:**
> [Brief explanation of what's causing the bug]
>
> **Affected components:**
> - [Component 1]
> - [Component 2]
>
> **Related files to review:**
> - `file1.rs` - [why relevant]
> - `file2.rs` - [why relevant]
>
> Does this match your understanding? Any additional context?

Wait for the user's response before proceeding.

---

## Step 4: Fix Approach

**Propose a fix approach:**

> ## Proposed Fix
>
> **Approach:**
> [Description of how to fix the bug]
>
> **Changes required:**
> 1. [File 1]: [Change description]
> 2. [File 2]: [Change description]
>
> **Testing strategy:**
> - [ ] Unit test for the specific bug case
> - [ ] Regression tests for related functionality
> - [ ] Manual verification steps
>
> **Risk assessment:**
> - Impact scope: [Low/Medium/High]
> - Regression risk: [Low/Medium/High]
>
> Does this approach look correct? Any concerns?

Wait for the user's response before proceeding.

---

## Step 5: Task Placement

**Analyze existing tasks and ask:**

> Where should this fix be placed in the task queue?
>
> **Options:**
> 1. **Immediate**: Add as next pending task (high priority)
> 2. **Current phase**: Add to end of current phase
> 3. **Specific phase**: Add to [phase-name]
> 4. **Deferred**: Add to backlog for later
>
> **Current phase:** [phase-name] ([N] pending tasks)
>
> Which priority level?

Wait for the user's response before proceeding.

---

## Step 6: Confirmation

**Present the complete fix task:**

> ## Fix Task Summary
>
> **Bug:** [summary]
> **Severity:** [severity]
> **Task ID:** [generated-task-id]
>
> **Task definition:**
> ```json
> {
>   "id": "[task-id]",
>   "name": "Fix: [summary]",
>   "type": "fix",
>   "status": "pending",
>   "depends_on": [],
>   "context": {
>     "files_to_read": ["affected/files.rs"],
>     "prior_review_issues": ["[bug description]"]
>   },
>   "instructions": "[detailed fix instructions]"
> }
> ```
>
> **Placement:** [phase and position]
>
> Does this look correct? Reply "yes" to add the fix task, or provide corrections.

Wait for explicit user confirmation before modifying files.

---

## Step 7: Update Project Files

Once confirmed, update the tasks.json file:

### 7.1 Generate Task ID

Use the format: `phase-X.fix-[short-name]` or `phase-X.task-N.fix` if fixing a specific task.

### 7.2 Add Fix Task to tasks.json

Insert the task in the appropriate position based on priority:
- **Immediate**: Insert at the beginning of pending tasks
- **Current phase**: Append to current phase
- **Specific phase**: Append to specified phase
- **Deferred**: Add with status "deferred"

### 7.3 Optionally Update ROADMAP.md

If the user wants, add a checkbox entry:

```markdown
- [ ] Fix: [bug summary]
```

---

## Step 8: Validate Changes

After updating the files, run validation to ensure all JSON files are correct:

```bash
cm --sanity-check
```

Check the output:
- If validation **passes**: Proceed to Step 9
- If validation **fails**:
  1. Review the error messages
  2. Fix the issues in tasks.json
  3. Re-run `cm --sanity-check`
  4. Repeat until all errors are resolved

**Common issues:**
- Duplicate task IDs
- Invalid depends_on references
- Missing required fields in new tasks

---

## Step 9: Completion

After updating files, inform the user:

> Fix task "[task-id]" has been added successfully!
>
> **Task details:**
> - ID: [task-id]
> - Name: Fix: [summary]
> - Phase: [phase-name]
> - Priority: [placement]
>
> **Files updated:**
> - `.cm/tasks.json` - Added fix task
>
> **Next steps:**
> 1. Validation completed in Step 8
> 2. Run `cm run` to execute tasks (fix will run at [position])
> 3. Alternatively, run `cm --step` to execute just this fix
>
> Would you like to add another bug fix?

---

## Fix Task Template

```json
{
  "id": "phase-X.fix-[name]",
  "name": "Fix: [bug summary]",
  "type": "fix",
  "status": "pending",
  "depends_on": [],
  "context": {
    "files_to_read": [
      "path/to/affected/file.rs"
    ],
    "prior_review_issues": [
      "Bug: [description]",
      "Observed: [behavior]",
      "Expected: [behavior]"
    ]
  },
  "instructions": "Fix [bug summary]:\n\n## Bug Details\n- Observed: [behavior]\n- Expected: [behavior]\n- Location: [file:line or function]\n\n## Root Cause\n[Explanation of why this bug occurs]\n\n## Fix Steps\n1. [Step 1]\n2. [Step 2]\n3. [Step 3]\n\n## Testing\n1. Add unit test for the bug case:\n   - Test input: [input]\n   - Expected output: [output]\n2. Verify existing tests still pass\n3. Run `cargo build` and `cargo clippy`\n\n## Verification\n- [ ] Bug no longer reproduces\n- [ ] No regressions in related functionality\n- [ ] All tests pass"
}
```

---

## Error Handling

If the wizard encounters issues:

### Missing Prerequisites
> I couldn't find `.cm/tasks.json`. Please run `/cm` first to initialize the project, then try `/fix` again.

### Invalid tasks.json
> The tasks.json file appears to be invalid. Please run `cm --validate` to check for errors.

### Conflicting Task IDs
> Task ID "[id]" already exists. I'll use "[new-id]" instead.

### No Active Phase
> No phase is currently active. I'll add the fix to phase "[first-pending-phase]".

---

## Tips for Effective Bug Fixes

1. **Reproduce first** - Ensure the bug can be reliably reproduced
2. **Root cause** - Fix the cause, not just the symptom
3. **Minimal changes** - Make the smallest change that fixes the bug
4. **Add tests** - Every bug fix should include a test
5. **Check regressions** - Verify related functionality still works
6. **Document** - Note why the fix works in code comments
