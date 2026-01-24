---
name: feat
description: Interactive wizard for adding new features to an existing cm project (project)
user-invocable: true
---

# Feature Wizard

This skill guides users through adding a new feature to an existing cm (Claude Code Manager) project. The wizard collects feature requirements through a conversational flow and then updates the project artifacts (PLAN.md, ROADMAP.md, tasks.json).

## Prerequisites

Before using this wizard, ensure:
- A `.cm/` directory exists with valid project files
- `tasks.json`, `PLAN.md`, and `ROADMAP.md` are present
- The project has been initialized with `/cm` or `cm init`

If prerequisites are not met, inform the user and suggest running `/cm` first.

## Important: Interactive Flow

You MUST follow this wizard flow step by step. Do NOT skip steps or modify files until you have gathered all the required information and received user confirmation.

---

## Step 1: Feature Overview

### Step 1.1: Feature Name

**Ask the user:**

> What is the **feature name**?
>
> (A short identifier like `user-auth`, `export-pdf`, `search-filter`)

Wait for the user's response before proceeding.

### Step 1.2: Feature Description

**Ask the user:**

> What does this feature do? (one sentence)

Wait for the user's response before proceeding.

### Step 1.3: User Story

**Ask the user:**

> What is the **user story**?
>
> Format: As a [user type], I want [goal] so that [benefit]

Wait for the user's response before proceeding.

---

## Step 2: Requirements Gathering

### Step 2.1: Functional Requirements

**Ask the user:**

> What should this feature **do**?
>
> (List the functional requirements)

Wait for the user's response before proceeding.

### Step 2.2: Non-Functional Requirements

**Ask the user:**

> Any **non-functional requirements**?
>
> (Performance, security, accessibility - or "none")

Wait for the user's response before proceeding.

### Step 2.3: User Interface

**Ask the user:**

> Any **UI/UX considerations**?
>
> (Interface requirements - or "N/A")

Wait for the user's response before proceeding.

### Step 2.4: Data Requirements

**Ask the user:**

> What **data** does this feature need?
>
> (Data storage, formats, sources - or "none specific")

Wait for the user's response before proceeding.

---

## Step 3: Technical Analysis

**Analyze the existing codebase:**

1. Read the current `.cm/PLAN.md` to understand project architecture
2. Read the current `.cm/tasks.json` to understand task structure
3. Identify which modules/components will be affected
4. Determine if new modules need to be created

**Ask the user:**

> Based on the project structure, here's my technical analysis:
>
> **Affected components:**
> - [List components that will be modified]
>
> **New components needed:**
> - [List new files/modules to create]
>
> **Dependencies:**
> - [List any new dependencies required]
>
> Do you have any additional technical requirements or constraints?

Wait for the user's response before proceeding.

---

## Step 4: Task Breakdown

**Ask the user:**

> Let's break this feature into implementation tasks.
>
> I suggest the following breakdown:
>
> 1. **[Task 1]** - [Description, estimated complexity]
> 2. **[Task 2]** - [Description, estimated complexity]
> 3. **[Task 3]** - [Description, estimated complexity]
> ...
>
> Each task should be:
> - Completable in a single agent session
> - Independently testable
> - Have clear success criteria
>
> Would you like to modify this breakdown?

Wait for the user's response before proceeding.

---

## Step 5: Integration Planning

### Step 5.1: Entry Points

**Ask the user:**

> Where will users **access** this feature?
>
> (UI locations, API endpoints, commands)

Wait for the user's response before proceeding.

### Step 5.2: Dependencies

**Ask the user:**

> Which existing **tasks must complete first**?
>
> (Dependencies from tasks.json - or "none")

Wait for the user's response before proceeding.

### Step 5.3: Testing

**Ask the user:**

> What **tests** are needed?
>
> (Unit tests, integration tests, manual testing)

Wait for the user's response before proceeding.

### Step 5.4: Documentation

**Ask the user:**

> What **documentation** updates are required?
>
> (README, API docs, user guides - or "none")

Wait for the user's response before proceeding.

---

## Step 6: Phase Placement

**Analyze existing phases in tasks.json and ask:**

> Where should this feature be placed in the project phases?
>
> **Current phases:**
> [List existing phases with their status]
>
> **Options:**
> 1. Add to existing phase: [phase-name]
> 2. Create new phase: [suggested-phase-name]
> 3. Create as a sub-phase after: [phase-name]
>
> Which option do you prefer?

Wait for the user's response before proceeding.

---

## Step 7: Confirmation

**Present a complete summary:**

> ## Feature Summary
>
> **Feature:** [name]
> **Description:** [description]
>
> **Tasks to add:**
> 1. [Task 1] - [type: implement/test/review]
> 2. [Task 2] - [type: implement/test/review]
> 3. [Task 3] - [type: implement/test/review]
>
> **Phase placement:** [phase info]
>
> **Dependencies:**
> - [dependency 1]
> - [dependency 2]
>
> **Files to update:**
> - `.cm/PLAN.md` - Add feature documentation
> - `.cm/ROADMAP.md` - Add feature tasks with checkboxes
> - `.cm/tasks.json` - Add task definitions
>
> Does this look correct? Reply "yes" to update the files, or provide corrections.

Wait for explicit user confirmation before modifying files.

---

## Step 8: Update Project Files

Once confirmed, update all three files:

### 8.1 Update PLAN.md

Add a new section for the feature under the appropriate module or create a new module section:

```markdown
### [Feature Name]

**Purpose:** [Feature description]

**Key files:**
- `path/to/file1.rs` - [Description]
- `path/to/file2.rs` - [Description]

**User story:** As a [user type], I want [goal] so that [benefit].

**Requirements:**
- [Requirement 1]
- [Requirement 2]
```

### 8.2 Update ROADMAP.md

Add checkboxes for the new tasks in the appropriate phase:

```markdown
### [Feature Name]

- [ ] [Task 1] - [Brief description]
- [ ] [Task 2] - [Brief description]
  - [ ] [Subtask 2.1]
  - [ ] [Subtask 2.2]
- [ ] [Task 3] - [Brief description]
```

Update the Summary table with new task counts.

### 8.3 Update tasks.json

Add new tasks to the appropriate phase:

```json
{
  "id": "phase-X.feat-[name].task-1",
  "name": "[Task Name]",
  "type": "implement",
  "status": "pending",
  "depends_on": ["previous-task-id"],
  "context": {
    "files_to_read": ["relevant/files.rs"],
    "code_style_excerpt": "Relevant style notes if any"
  },
  "instructions": "Detailed implementation instructions..."
}
```

---

## Step 9: Regenerate Markdown

After updating the JSON files, regenerate the markdown documentation:

```bash
cm --regenerate
```

This ensures ROADMAP.md and LOG.md stay in sync with the JSON source files (roadmap.json, tasks.json).

---

## Step 10: Validate Changes

After updating the files, run validation to ensure all JSON files are correct:

```bash
cm --sanity-check
```

Check the output:
- If validation **passes**: Proceed to Step 11
- If validation **fails**:
  1. Review the error messages
  2. Fix the issues in the JSON files (tasks.json or roadmap.json)
  3. Re-run `cm --sanity-check`
  4. Repeat until all errors are resolved

**Common issues:**
- Invalid cross-references (roadmap_item_id pointing to non-existent item)
- Duplicate task IDs
- Missing required fields in new tasks

---

## Step 11: Completion

After updating all files, inform the user:

> Feature "[feature-name]" has been added successfully!
>
> **Updated files:**
> - `.cm/PLAN.md` - Added feature documentation
> - `.cm/ROADMAP.md` - Added [N] new tasks
> - `.cm/tasks.json` - Added [N] task definitions
>
> **New tasks added:**
> - [task-id-1]: [task-name-1]
> - [task-id-2]: [task-name-2]
> - [task-id-3]: [task-name-3]
>
> **Next steps:**
> 1. Review the updated files to ensure accuracy
> 2. Run `cm run` to start executing tasks
>
> Would you like to add another feature or make any adjustments?

---

## Task Templates

### Implementation Task Template

```json
{
  "id": "phase-X.feat-[name].impl-[n]",
  "name": "Implement [component]",
  "type": "implement",
  "status": "pending",
  "depends_on": [],
  "context": {
    "files_to_read": [],
    "code_style_excerpt": null
  },
  "instructions": "Implement [component] for [feature]:\n\n1. Create [file path]\n2. Implement [function/struct] that:\n   - [Requirement 1]\n   - [Requirement 2]\n3. Add error handling for:\n   - [Error case 1]\n   - [Error case 2]\n4. Ensure `cargo build` and `cargo clippy` pass"
}
```

### Test Task Template

```json
{
  "id": "phase-X.feat-[name].test-[n]",
  "name": "Test [component]",
  "type": "test",
  "status": "pending",
  "depends_on": ["phase-X.feat-[name].impl-[n]"],
  "context": {
    "files_to_read": ["src/component.rs"]
  },
  "instructions": "Add tests for [component]:\n\n1. Add unit tests in [file]:\n   - Test [scenario 1]\n   - Test [scenario 2]\n   - Test error handling for [case]\n2. Add integration tests if needed\n3. Ensure `cargo test` passes with all new tests"
}
```

### Review Task Template

```json
{
  "id": "phase-X.feat-[name].review",
  "name": "Review [feature] implementation",
  "type": "review",
  "status": "pending",
  "depends_on": ["phase-X.feat-[name].impl-1", "phase-X.feat-[name].test-1"],
  "context": {
    "files_to_read": ["src/feature/"]
  },
  "instructions": "Review [feature] implementation:\n\n1. Check code quality and style consistency\n2. Verify all requirements are met\n3. Check error handling completeness\n4. Review test coverage\n5. Document any issues found for fixing"
}
```

---

## Tips for Good Feature Tasks

1. **Atomic tasks** - Each task should do one thing well
2. **Clear dependencies** - Ensure proper task ordering
3. **Testable outcomes** - Each task should have verifiable results
4. **Context matters** - Include relevant files in `files_to_read`
5. **Detailed instructions** - Be specific about what to implement
6. **Error handling** - Always include error cases in requirements
7. **Build verification** - End tasks with build/test verification

---

## Error Handling

If the wizard encounters issues:

### Missing Prerequisites
> I couldn't find `.cm/tasks.json`. Please run `/cm` first to initialize the project, then try `/feat` again.

### Invalid tasks.json
> The tasks.json file appears to be invalid. Please run `cm --validate` to check for errors.

### Conflicting Task IDs
> Task ID "[id]" already exists. I'll use "[new-id]" instead.

### Phase Not Found
> Phase "[phase-id]" not found. Available phases: [list phases]. Which phase should I use?
