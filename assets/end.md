# End Session Wizard

This command finalizes a `/feat` or `/fix` conversational session by saving all gathered information to the planning files without starting implementation. Use this when you've discussed a feature or fix with the user and want to save it to the project plan for later execution.

## Prerequisites

Before using this command, ensure:
- You are completing a `/feat` or `/fix` session
- A `.cm/` directory exists with valid project files
- The user has confirmed all details about the feature/fix
- All required information has been gathered

If prerequisites are not met, inform the user.

## CRITICAL: Scope Limitations

This command ONLY updates planning files:
- `.cm/tasks.json` - Add new task definitions
- `.cm/roadmap.json` - Add new roadmap items
- `.cm/PLAN.md` - Add feature documentation (for `/feat` sessions)

This command does NOT:
- Implement any code
- Run `cm run` or execute tasks
- Make changes outside `.cm/` directory

After this command completes, the user must manually run `cm run` when ready to implement.

## Important: Session Context

This command assumes you have already:
- Gathered all feature/fix requirements from the user
- Confirmed task breakdown and approach
- Received user approval for the plan
- Identified phase placement for new tasks

DO NOT use this command to start a new planning session. Use `/feat` or `/fix` for that.

---

## Step 1: Recognize Session Type

Determine which type of session you're finalizing:

**For /feat sessions:**
- Review the feature name, description, and requirements gathered
- Identify all tasks that were discussed
- Note phase placement decisions
- Confirm what should be added to PLAN.md

**For /fix sessions:**
- Review the bug description and severity
- Identify the fix task that was discussed
- Note task placement and priority
- No PLAN.md updates needed (fixes don't modify architectural docs)

Confirm with the user:

> I'm going to save the [feature/fix] we discussed to the planning files.
>
> **Summary:**
> - [Feature name/Bug summary]
> - [N] tasks to add
> - Phase: [phase-name]
>
> Proceed? (yes/no)

Wait for confirmation before proceeding.

---

## Step 2: Update tasks.json

Add the new task definitions to `.cm/tasks.json`:

### 2.1 Read Current tasks.json

Read the current `.cm/tasks.json` file to:
- Find the appropriate phase to add tasks to
- Check for ID conflicts
- Ensure dependencies are valid

### 2.2 Add New Tasks

Insert new tasks following the schema:

```json
{
  "id": "phase-X.feat-[name].task-1",
  "name": "Task Name",
  "type": "implement",
  "status": "pending",
  "depends_on": [],
  "context": {
    "files_to_read": ["relevant/files.rs"],
    "code_style_excerpt": null
  },
  "instructions": "Detailed implementation instructions..."
}
```

**Task ID Format:**
- For features: `phase-X.feat-[name].task-N`
- For fixes: `phase-X.fix-[name]` or `phase-X.task-N.fix`

**Required Fields:**
- `id`: Unique task identifier
- `name`: Human-readable task name
- `type`: One of "implement", "review", "fix", "test"
- `status`: Should be "pending" for new tasks
- `context`: Must include at minimum an empty object
- `instructions`: Detailed instructions for the agent

**Dependencies:**
- Ensure `depends_on` references only existing task IDs
- Tasks can only depend on tasks from the same phase or earlier phases
- Do not create circular dependencies

### 2.3 Add roadmap_item_id Links

For each task, if it corresponds to a roadmap item, add the `roadmap_item_id` field to link them:

```json
{
  "id": "phase-1.task-1",
  "roadmap_item_id": "item-1",
  ...
}
```

This ensures roadmap checkboxes sync with task completion.

---

## Step 3: Update roadmap.json

Add new roadmap items to `.cm/roadmap.json`:

### 3.1 Read Current roadmap.json

Read the current `.cm/roadmap.json` file to:
- Find the appropriate phase
- Generate unique item IDs
- Ensure consistency

### 3.2 Add New Items

Insert new roadmap items following the schema:

```json
{
  "id": "item-X",
  "name": "Item Name",
  "completed": false,
  "sub_items": [
    { "name": "Sub-item name", "completed": false }
  ],
  "linked_task_ids": ["phase-1.task-1", "phase-1.task-2"]
}
```

**Item Structure:**
- `id`: Unique identifier for the item
- `name`: Human-readable name (matches task names or groups them)
- `completed`: Always `false` for new items
- `sub_items`: Optional array of sub-items with checkboxes
- `linked_task_ids`: Array of task IDs that this item represents

**Linking Strategy:**
- One roadmap item can represent multiple tasks
- Use `linked_task_ids` to track which tasks complete this item
- The item is marked completed when ALL linked tasks are completed

---

## Step 4: Update PLAN.md (Features Only)

**ONLY for /feat sessions:** Add feature documentation to `.cm/PLAN.md`.

**SKIP this step for /fix sessions** - bug fixes do not require PLAN.md updates.

### 4.1 Determine Placement

Analyze the PLAN.md structure and determine where to add the feature:
- Under an existing module section
- As a new module section
- As a subsection of an existing feature

### 4.2 Add Feature Documentation

Add a new section with this structure:

```markdown
### [Feature Name]

**Purpose:** [Feature description]

**Key files:**
- `path/to/file1.rs` - [Description]
- `path/to/file2.rs` - [Description]

**Requirements:**
- [Requirement 1]
- [Requirement 2]

**Implementation notes:**
- [Note 1]
- [Note 2]
```

Follow the existing style and formatting in PLAN.md.

---

## Step 5: Regenerate Markdown Files

After updating the JSON files, regenerate the markdown documentation to ensure synchronization:

```bash
cm --regenerate
```

This command:
- Regenerates `ROADMAP.md` from `roadmap.json`
- Regenerates `LOG.md` from `tasks.json` log records
- Updates summary tables and progress counts

**Expected output:**
```
Regenerating ROADMAP.md from roadmap.json...
Regenerating LOG.md from tasks.json...
✓ Files regenerated successfully
```

If the command fails, investigate the error and fix any JSON formatting issues.

---

## Step 6: Validate Changes (MANDATORY GATE)

After updating all files, you MUST run validation and fix ALL issues before proceeding.

### 6.1 Run sanity check

```bash
cm --sanity-check
```

This checks:
- JSON syntax is valid
- All task IDs are unique
- All dependencies reference existing tasks
- All `roadmap_item_id` references point to valid items
- All `linked_task_ids` in roadmap point to valid tasks
- Every uncompleted roadmap item has `linked_task_ids` pointing to tasks
- Required fields are present

### 6.2 If sanity check fails: FIX and RE-RUN

You MUST loop until the sanity check passes:

1. Read the error/warning output carefully
2. Fix the issues in the JSON files (tasks.json and/or roadmap.json)
3. Re-run `cm --sanity-check`
4. **Repeat from step 1 until ALL errors are resolved**

**Common issues and fixes:**
- **Duplicate task IDs**: Change one of the conflicting IDs
- **Invalid cross-references**: Ensure `roadmap_item_id` points to an existing roadmap item ID
- **Missing required fields**: Add the missing fields to task definitions
- **Invalid dependencies**: Check that `depends_on` references existing task IDs
- **Circular dependencies**: Remove dependency loops
- **Uncompleted roadmap item with no linked tasks**: Add `linked_task_ids` to the roadmap item pointing to the task(s) you created, OR add a new task in tasks.json and link it

### 6.3 Also check warnings

Warnings (e.g., orphaned roadmap items) indicate roadmap items that no task will ever complete. For each warning:
- If the roadmap item should be completed by a task you just added, add its task ID to `linked_task_ids`
- If the roadmap item needs a NEW task, go back to Step 2 and add one

**DO NOT proceed to Step 7 until `cm --sanity-check` reports zero errors AND zero warnings.**

---

## Step 7: Completion Summary

After all files are updated and validated, inform the user:

> **Session finalized successfully!**
>
> **Changes saved:**
> - `.cm/tasks.json` - Added [N] new task(s)
> - `.cm/roadmap.json` - Added [M] new roadmap item(s)
> [For /feat only:] - `.cm/PLAN.md` - Added [feature name] documentation
>
> **New tasks added:**
> - `[task-id-1]`: [task-name-1]
> - `[task-id-2]`: [task-name-2]
> - `[task-id-3]`: [task-name-3]
>
> **Validation:** ✓ All checks passed
>
> **Next steps:**
> 1. Review the updated files to ensure accuracy
> 2. When ready to implement, run `cm run` to start executing tasks
> 3. Or run `cm --step` to execute tasks one at a time
>
> The feature/fix has been saved to your planning files and is ready for implementation whenever you choose to run it.

---

## Error Handling

If the wizard encounters issues:

### Missing .cm/ Directory

> I couldn't find the `.cm/` directory. This skill requires an initialized cm project.
>
> Please run `/cm` first to initialize the project, then return to your `/feat` or `/fix` session.

### Invalid tasks.json

> The current `tasks.json` file appears to be invalid or corrupted.
>
> Please run `cm --sanity-check` to identify issues, then fix them before using `/end`.

### Conflicting Task IDs

> Task ID "[id]" already exists in tasks.json. I'll use "[new-id]" instead.
>
> **Original:** [id]
> **Updated:** [new-id]
>
> Is this acceptable? (yes/no)

Wait for user confirmation before proceeding.

### Invalid Phase Reference

> Phase "[phase-id]" not found in tasks.json.
>
> Available phases:
> - [phase-1-id]: [phase-1-name]
> - [phase-2-id]: [phase-2-name]
>
> Which phase should I add the tasks to?

Wait for user to select a valid phase.

### Invalid Roadmap Item ID

> Roadmap item ID "[id]" doesn't exist in roadmap.json.
>
> The task references this item via `roadmap_item_id`, but it's not defined.
>
> Options:
> 1. Remove the `roadmap_item_id` field (no linking)
> 2. Create the roadmap item "[id]"
>
> Which option? (1/2)

Wait for user choice.

### Cross-Reference Validation Failures

If `cm --sanity-check` reports cross-reference errors:

> **Validation failed with cross-reference errors:**
>
> [Display error messages from cm --sanity-check]
>
> **Fixing issues:**
> - [Describe fix for error 1]
> - [Describe fix for error 2]
>
> I'll update the files to resolve these issues...

Then fix the errors and re-run validation until it passes.

---

## Schema References

### tasks.json Task Schema

```json
{
  "id": "phase-X.task-N",
  "name": "Task Name",
  "type": "implement|review|fix|test",
  "status": "pending|in_progress|completed|deferred",
  "depends_on": ["task-id-1", "task-id-2"],
  "context": {
    "files_to_read": ["file1.rs", "file2.rs"],
    "code_style_excerpt": "Optional style notes",
    "prior_review_issues": ["Issue 1", "Issue 2"]
  },
  "instructions": "Detailed instructions for the agent",
  "roadmap_item_id": "optional-item-id"
}
```

### roadmap.json Item Schema

```json
{
  "id": "item-X",
  "name": "Item Name",
  "completed": false,
  "sub_items": [
    { "name": "Sub-item name", "completed": false }
  ],
  "linked_task_ids": ["phase-1.task-1"]
}
```

### Task ID Naming Conventions

- **Feature tasks:** `phase-X.feat-[feature-name].task-N`
- **Fix tasks:** `phase-X.fix-[bug-name]` or `phase-X.task-N.fix`
- **Review tasks:** `phase-X.task-N.review`
- **Test tasks:** `phase-X.feat-[name].test-N` or `phase-X.task-N.test`

### Dependency Rules

1. Tasks can depend on tasks from the same phase or earlier phases only
2. No circular dependencies allowed
3. Dependencies must reference existing task IDs
4. Use dependencies to enforce implementation order (e.g., test depends on implement)

---

## Tips for Effective Session Finalization

1. **Verify completeness** - Ensure all discussed tasks are included
2. **Check consistency** - Task names and descriptions should match what was discussed
3. **Validate dependencies** - Make sure task ordering makes sense
4. **Link roadmap items** - Use `roadmap_item_id` to keep checkboxes in sync
5. **Test validation** - Always run `cm --sanity-check` before finishing
6. **Clear summary** - Provide a comprehensive summary so the user knows what was saved
7. **Review opportunity** - Remind users to review files before running `cm run`

---

## Example Usage

### Example 1: Finalizing a /feat Session

```
User: I think we've covered everything for the export feature. Can you save this to the planning files?

Agent: I'm going to save the export feature we discussed to the planning files.

**Summary:**
- Export feature with CSV, JSON, and PDF formats
- 3 tasks to add (implement, test, review)
- Phase: phase-2 (Core Features)

Proceed? (yes/no)

User: yes

Agent: [Reads tasks.json and roadmap.json]
Agent: [Adds 3 new tasks to phase-2 in tasks.json]
Agent: [Adds roadmap items with linked_task_ids]
Agent: [Updates PLAN.md with export feature documentation]
Agent: [Runs cm --regenerate]
Agent: [Runs cm --sanity-check - passes]

**Session finalized successfully!**

**Changes saved:**
- `.cm/tasks.json` - Added 3 new tasks
- `.cm/roadmap.json` - Added 1 new roadmap item
- `.cm/PLAN.md` - Added export feature documentation

**New tasks added:**
- `phase-2.feat-export.impl-1`: Implement CSV export
- `phase-2.feat-export.impl-2`: Implement JSON export
- `phase-2.feat-export.impl-3`: Implement PDF export

**Validation:** ✓ All checks passed

**Next steps:**
1. Review the updated files to ensure accuracy
2. When ready to implement, run `cm run` to start executing tasks
3. Or run `cm --step` to execute tasks one at a time

The feature has been saved to your planning files and is ready for implementation whenever you choose to run it.
```

### Example 2: Finalizing a /fix Session

```
User: Let's save this fix for the memory leak to the tasks.

Agent: I'm going to save the fix we discussed to the planning files.

**Summary:**
- Fix memory leak in connection pool
- 1 task to add
- Phase: phase-1 (immediate priority)

Proceed? (yes/no)

User: yes

Agent: [Reads tasks.json and roadmap.json]
Agent: [Adds fix task to phase-1 in tasks.json]
Agent: [Adds roadmap item for the fix]
Agent: [Skips PLAN.md - not needed for fixes]
Agent: [Runs cm --regenerate]
Agent: [Runs cm --sanity-check - passes]

**Session finalized successfully!**

**Changes saved:**
- `.cm/tasks.json` - Added 1 new task
- `.cm/roadmap.json` - Added 1 new roadmap item

**New tasks added:**
- `phase-1.fix-memory-leak`: Fix memory leak in connection pool

**Validation:** ✓ All checks passed

**Next steps:**
1. Review the updated files to ensure accuracy
2. When ready to implement, run `cm run` to start executing tasks
3. Or run `cm --step` to execute tasks one at a time

The fix has been saved to your planning files and is ready for implementation whenever you choose to run it.
```

---

## Summary

The `/end` command bridges the gap between planning and execution by:
1. Capturing all details from /feat or /fix sessions
2. Updating planning files (tasks.json, roadmap.json, PLAN.md)
3. Ensuring consistency with validation
4. Providing clear summary of what was saved

This allows users to have thoughtful planning conversations, save the results, and execute them later when ready - without losing context or requiring re-discussion.