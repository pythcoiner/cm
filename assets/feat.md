---
name: feat
description: Interactive wizard for adding new features to an existing cm project
(project)
user-invocable: true
---

# Feature Wizard

This skill guides users through adding a new feature to an existing cm (Claude Code
Manager) project. The wizard collects feature requirements through a conversational
flow and then updates the project artifacts (PLAN.md, ROADMAP.md, tasks.json).

## Prerequisites

Before using this wizard, ensure:
- A `.cm/` directory exists with valid project files
- `tasks.json`, `PLAN.md`, and `ROADMAP.md` are present
- The project has been initialized with `/cm` or `cm init`

If prerequisites are not met, inform the user and suggest running `/cm` first.

## CRITICAL: Scope Limitations

This skill ONLY updates planning files:
- `.cm/PLAN.md` - Add feature documentation
- `.cm/roadmap.json` - Add roadmap items
- `.cm/tasks.json` - Add task definitions

This skill does NOT:
- Implement any code
- Run `cm run` or execute tasks
- Make changes outside `.cm/` directory

After the wizard completes, the user must manually run `cm run` to start
implementation.

## Important: Interactive Flow

You MUST follow this wizard flow step by step. Do NOT skip steps or modify files
until you have gathered all the required information and received user confirmation.

---

## Step 1: Feature Description

**Ask the user:**

> **Describe the feature you want to add.**
>
> Include:
> - What it does
> - Why it's needed
> - Any key requirements

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

## Step 8: Handoff to /end

After the user confirms the feature summary, inform them:

> The feature plan is ready. To save these changes to the project files, run `/end`.
>
> This will update:
> - `.cm/tasks.json` - Add task definitions
> - `.cm/roadmap.json` - Add roadmap items
> - `.cm/PLAN.md` - Add feature documentation
>
> After saving, run `cm run` when ready to start implementation.

Do NOT modify any files. Wait for the user to run `/end`.

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
  "instructions": "Implement [component] for [feature]:\n\n1. Create [file path]\n2.
Implement [function/struct] that:\n   - [Requirement 1]\n   - [Requirement 2]\n3.
Add error handling for:\n   - [Error case 1]\n   - [Error case 2]\n4. Ensure `cargo
build` and `cargo clippy` pass"
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
  "instructions": "Add tests for [component]:\n\n1. Add unit tests in [file]:\n   -
Test [scenario 1]\n   - Test [scenario 2]\n   - Test error handling for [case]\n2.
Add integration tests if needed\n3. Ensure `cargo test` passes with all new tests"
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
  "instructions": "Review [feature] implementation:\n\n1. Check code quality and
style consistency\n2. Verify all requirements are met\n3. Check error handling
completeness\n4. Review test coverage\n5. Document any issues found for fixing"
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
> I couldn't find `.cm/tasks.json`. Please run `/cm` first to initialize the
project, then try `/feat` again.

### Invalid tasks.json
> The tasks.json file appears to be invalid. Please run `cm --validate` to check for
errors.

### Conflicting Task IDs
> Task ID "[id]" already exists. I'll use "[new-id]" instead.

### Phase Not Found
> Phase "[phase-id]" not found. Available phases: [list phases]. Which phase should
I use?
