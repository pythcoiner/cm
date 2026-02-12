# Clone - Source Code Specification Generator

This command analyzes a codebase and generates a comprehensive, implementation-ready
specification. The output is a set of detailed markdown documents that describe the
application precisely enough to rebuild it from scratch.

**Primary focus: GUI applications.** CLI applications are supported as a secondary
target.

## Important: Interactive Flow

You MUST follow this flow step by step. Do NOT skip steps or generate files until
you have gathered all required information and received user confirmation. Read the
ENTIRE codebase before writing any specification document.

---

## Step 1: Initial Codebase Survey

**Explore the project systematically:**

1. Use `Glob` to discover all source files (e.g.,
`**/*.{py,rs,ts,js,go,java,cpp,c,rb,php,cs,swift,kt,qml}`)
2. Read the project manifest (`Cargo.toml`, `package.json`, `pyproject.toml`,
`go.mod`, `CMakeLists.txt`, `*.pro`, etc.)
3. Read `README.md` if it exists
4. Read the entry point file(s) (`main.*`, `app.*`, `index.*`, etc.)
5. Identify the UI framework (Qt, GTK, Electron, Tauri, SwiftUI, WPF, Tkinter, etc.)
6. Count source files and identify the primary language

**Present your findings to the user:**

> **Codebase Survey Results:**
>
> - **Project:** [name]
> - **Language:** [primary language]
> - **UI Framework:** [framework] *(or "CLI application - no GUI framework
detected")*
> - **Source files:** [count] files across [count] directories
> - **Entry point:** [file path]
> - **Key dependencies:** [list]
>
> Does this look correct? Should I analyze the full codebase or focus on specific
modules?

Wait for the user's response before proceeding.

---

## Step 2: Project Type Classification

Based on your survey, determine the project type:

**If GUI application** (default):
- 4th document will be `VIEWS.md` (UI layout, widgets, visual specifications)

**If CLI application:**
- 4th document will be `CLI_INTERFACE.md` (commands, flags, output formats, exit
codes)

**Inform the user and proceed immediately to deep analysis:**

> Based on my analysis, this is a **[GUI/CLI]** application.
> I will generate these specification documents in the `cloned/` directory:
>
> 1. `FEATURES.md` - Feature specifications and algorithms
> 2. `ACTIONS.md` - User actions and execution chains
> 3. `DATA_MODELS.md` - Data structures, configuration, and constants
> 4. `[VIEWS.md or CLI_INTERFACE.md]` - [UI specifications / CLI interface
specifications]
>
> Starting deep analysis...

Do NOT wait for confirmation. Proceed directly to Step 3.

---

## Step 3: Deep Codebase Analysis

**Read ALL source files in the project** (or the agreed scope). Organize your
understanding by document concern:

### For FEATURES.md:
- Identify every feature/capability the application provides
- Trace algorithms and their step-by-step logic
- Find key functions: their purpose, inputs, outputs, and side effects
- Note threading model, async patterns, and concurrency
- Document error handling strategies
- Identify configuration options and their effects

### For ACTIONS.md:
- Find all user-initiated actions (button clicks, menu items, keyboard shortcuts,
CLI commands)
- Trace each action's complete execution chain from trigger to completion
- Map signal/slot connections, event handlers, callbacks
- Document state changes triggered by each action
- Note preconditions (locks, enabled states, validation)
- Track cross-references between actions (e.g., commit+push calls commit then push)

### For DATA_MODELS.md:
- Extract all data structures, types, enums, interfaces
- Document configuration file schemas with field specifications
- List all constants, magic numbers, hardcoded values
- Catalog status messages (success and error)
- Document icon/asset files and their purposes
- Note timing constants, thresholds, limits

### For VIEWS.md (GUI):
- Document every widget/component: type, position, size, properties
- Map the complete layout with coordinates or layout hierarchy
- Document color palette/theme
- List all icons and their assignments
- Document modal dialogs with their content and triggers
- Note visibility conditions and dynamic states
- Document extended/collapsed modes if any

### For CLI_INTERFACE.md (CLI):
- Document every command and subcommand
- List all flags/options with types, defaults, and descriptions
- Document output formats (table, JSON, plain text)
- List all exit codes and their meanings
- Document interactive prompts if any
- Note environment variables that affect behavior

Do NOT present analysis results to the user. Proceed directly to document generation.

### Sub-Agent Strategy

**Use sub-agents (Task tool) to perform the deep analysis.** Reading the entire
codebase in the main agent context will bloat it and degrade output quality. Instead:

1. Launch sub-agents in parallel to read and analyze source files
2. Each sub-agent should focus on one document's concerns (e.g., one agent for
FEATURES, one for ACTIONS, one for DATA_MODELS, one for VIEWS)
3. Each sub-agent returns a structured summary of its findings
4. The main agent uses these summaries to write the final documents

This keeps the main agent context clean and focused on generation rather than raw
source code.

---

## Step 4: Generate Documents

Create the `cloned/` directory at the repository root if it doesn't exist, then
generate documents in this order:

1. `cloned/DATA_MODELS.md` — foundation (types, constants referenced by other docs)
2. `cloned/FEATURES.md` — builds on data models
3. `cloned/ACTIONS.md` — builds on features and data models
4. `cloned/VIEWS.md` or `cloned/CLI_INTERFACE.md` — references all three above

After generation, inform the user:

> Specification generated in `cloned/`:
> - `DATA_MODELS.md` - [line count] lines
> - `FEATURES.md` - [line count] lines
> - `ACTIONS.md` - [line count] lines
> - `[4th doc]` - [line count] lines

---

## Formatting Rules

**ALL specification content MUST use ASCII box tables inside markdown code blocks.**
This is the core formatting convention. Never use plain markdown tables or
bullet-point descriptions for specifications.

### Function/Feature Specification Box

```
+----------------------------------------------------------+
| Function: function_name()                                |
+----------------------------------------------------------+
| Purpose: Brief description of what this function does    |
+----------------------------------------------------------+
| Algorithm:                                               |
| 1. First step                                            |
|    a. Sub-step with detail                               |
|    b. Another sub-step                                   |
| 2. Second step                                           |
|    a. Sub-step                                           |
+----------------------------------------------------------+
| Output: Return type or side effect description           |
+----------------------------------------------------------+
| Note: Any important caveats or edge cases                |
+----------------------------------------------------------+
```

Rules:
- Box width is consistently 58 characters inner width (60 with borders)
- All content lines are left-padded with `| ` and right-padded with spaces to reach
` |`
- Section dividers use `+--...--+` lines
- Use `Function:`, `Purpose:`, `Algorithm:`, `Process:`, `Output:`, `Note:`,
`Command:` labels

### Execution Chain Diagram (ACTIONS.md)

```
+------------------------------------------------------------------+
| Widget: widget_name (WidgetType)                                 |
| Event: event_name                                                |
| Connection: signal.connect(slot)                                 |
+------------------------------------------------------------------+
| Function Called: handler_function()                               |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. handler_function()                                            |
|    |                                                             |
|    +-> first_call()                                              |
|    +-> second_call()                                             |
|                                                                  |
| 2. first_call()                                                  |
|    |                                                             |
|    +-> IF condition:                                             |
|    |   +-> do_something()                                        |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> ELSE:                                                     |
|        +-> do_other_thing()                                      |
|        +-> FOR each item in list:                                |
|            +-> process(item)                                     |
+------------------------------------------------------------------+
```

Rules:
- Box width is 66 characters inner width (68 with borders) for action specs
- `+->` for function calls and flow steps
- `|` for vertical continuation lines
- IF/ELSE/FOR/WHILE control flow inline with proper indentation
- Indentation increases by 4 spaces per nesting level
- Number each function in the chain (1, 2, 3...)
- Show thread context in brackets: `[in thread]`, `[on signal]`

### Property Table

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | WidgetType          |
| Object Name       | widget_name         |
| X Position        | 10                  |
| Y Position        | 40                  |
| Width             | 271                 |
| Height            | 231                 |
+-------------------+---------------------+
```

### Two-Column Reference Table

```
+---------------------------+------------------------------+
| Key                       | Description                  |
+---------------------------+------------------------------+
| item_one                  | What this item is            |
| item_two                  | What this item is            |
+---------------------------+------------------------------+
```

### Cross-References

- Between documents: `(See DATA_MODELS.md section 2.3)`
- Within documents: `(See section 1.2)`, `(See execution chain above)`
- Forward references when the target hasn't been written yet: `(Defined in VIEWS.md)`

### Section Numbering

- Top-level: `## 1. Section Name`
- Sub-sections: `### 1.1 Sub-section Name`
- Separate major sections with `---` horizontal rules
- Within boxes: numbered steps (1, 2, 3) with lettered sub-steps (a, b, c)

---

## Document Templates

### FEATURES.md Template

```markdown
# [Application Name] Features Specification

## 1. [First Major Feature Area]

### 1.1 [Specific Feature]

` ` `
+----------------------------------------------------------+
| Function: relevant_function()                            |
+----------------------------------------------------------+
| Purpose: What this feature does                          |
+----------------------------------------------------------+
| Algorithm:                                               |
| 1. Step one                                              |
|    a. Detail                                             |
| 2. Step two                                              |
+----------------------------------------------------------+
| Output: What is produced                                 |
+----------------------------------------------------------+
` ` `

### 1.2 [Next Feature]

...

---

## 2. [Second Major Feature Area]

...
```

### ACTIONS.md Template

```markdown
# [Application Name] User Actions Specification

## 1. Button Actions

### 1.1 [Button Name] (`widget_name`)

` ` `
+------------------------------------------------------------------+
| Widget: widget_name (WidgetType)                                 |
| Event: clicked                                                   |
| Connection: signal.connect(slot)                                 |
+------------------------------------------------------------------+
| Function Called: handler()                                        |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. handler()                                                     |
|    |                                                             |
|    +-> step_one()                                                |
|    +-> step_two()                                                |
+------------------------------------------------------------------+
` ` `

---

## 2. Selection Actions

...

## 3. Keyboard Actions

...

## 4. Timer Actions

...

## 5. Signal/Slot Connections Summary

` ` `
+------------------------------------------------------------------+
| Signal                          | Slot                           |
+---------------------------------+--------------------------------+
| widget.signal                   | handler()                      |
+---------------------------------+--------------------------------+
` ` `

## 6. Button State Reference

...
```

### DATA_MODELS.md Template

```markdown
# [Application Name] Constants & Configuration Specification

## 1. Configuration Schema

### 1.1 [Format] Format

` ` `
+----------------------------------------------------------+
| File: config_file_name                                   |
+----------------------------------------------------------+
| [Schema definition with types and constraints]           |
+----------------------------------------------------------+
` ` `

### 1.2 Field Specifications

` ` `
+----------------------------------------------------------+
| Field: field_name                                        |
+----------------------------------------------------------+
| Type: field_type                                         |
| Required: Yes/No                                         |
| Default: default_value                                   |
| Description: What this field controls                    |
+----------------------------------------------------------+
` ` `

---

## 2. Constants

### 2.1 [Constant Category]

` ` `
+----------------------------------------------------------+
| [Description of constant group]                          |
+----------------------------------------------------------+
| CONSTANT_NAME        | value or description               |
+----------------------------------------------------------+
` ` `

---

## 3. [Asset Files / Icons / Resources]

...

## 4. Status Messages

### 4.1 Success Messages

` ` `
+-------------------------+--------------------------------+
| Operation               | Message                        |
+-------------------------+--------------------------------+
| Operation name          | "Success message"              |
+-------------------------+--------------------------------+
` ` `

### 4.2 Error Messages

...
```

### VIEWS.md Template (GUI Applications)

```markdown
# [Application Name] Views Specification (Pixel-Perfect)

## 1. Main Window Properties

` ` `
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Title             | "App Title"         |
| Width             | X px                |
| Height            | Y px                |
| Resizable         | Yes/No              |
+-------------------+---------------------+
` ` `

---

## 2. Color Palette / Theme

` ` `
+---------------------------+-------------------+-------------+
| Element                   | Role              | Value       |
+---------------------------+-------------------+-------------+
| Background                | Window            | #RRGGBB     |
+---------------------------+-------------------+-------------+
` ` `

---

## 3. Widget Specifications

### 3.1 [Widget Name] (`object_name`)

` ` `
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | WidgetType          |
| Object Name       | object_name         |
| X Position        | N                   |
| Y Position        | N                   |
| Width             | N                   |
| Height            | N                   |
| Text              | "label"             |
| Icon              | icon_file.png       |
| Tooltip           | "tooltip text"      |
+-------------------+---------------------+
` ` `

[Repeat for every widget]

---

## 4. Complete Layout Grid

` ` `
[ASCII art showing spatial arrangement of all widgets
with pixel coordinate labels on Y axis and X positions marked]
` ` `

---

## 5. Widget Summary Table

` ` `
+------------------+------+------+-------+--------+
| Widget           | X    | Y    | Width | Height |
+------------------+------+------+-------+--------+
| widget_name      | N    | N    | N     | N      |
+------------------+------+------+-------+--------+
` ` `

---

## 6. Modal Dialogs

### 6.1 [Dialog Name]

` ` `
+------------------------------------------+
| Type              | DialogType          |
| Title             | "Dialog Title"      |
| Text              | "Dialog message"    |
| Buttons           | OK / Cancel / etc.  |
| Trigger           | condition           |
+-------------------+---------------------+
` ` `
```

### CLI_INTERFACE.md Template (CLI Applications)

```markdown
# [Application Name] CLI Interface Specification

## 1. Command Overview

` ` `
+----------------------------------------------------------+
| Binary: command_name                                     |
+----------------------------------------------------------+
| Usage: command_name [OPTIONS] [COMMAND]                   |
+----------------------------------------------------------+
` ` `

---

## 2. Global Options

` ` `
+----------------------------------------------------------+
| Flag: --option-name                                      |
+----------------------------------------------------------+
| Short: -o                                                |
| Type: string/bool/int                                    |
| Required: Yes/No                                         |
| Default: default_value                                   |
| Description: What this option does                       |
+----------------------------------------------------------+
` ` `

---

## 3. Commands

### 3.1 [Command Name]

` ` `
+----------------------------------------------------------+
| Command: command_name subcommand                         |
+----------------------------------------------------------+
| Purpose: What this command does                          |
+----------------------------------------------------------+
| Arguments:                                               |
| - arg1 (type): description                               |
+----------------------------------------------------------+
| Options:                                                 |
| - --flag (type): description [default: value]            |
+----------------------------------------------------------+
| Output:                                                  |
| [Description of stdout output format]                    |
+----------------------------------------------------------+
| Exit Codes:                                              |
| 0 - Success                                              |
| 1 - Error description                                    |
+----------------------------------------------------------+
` ` `

---

## 4. Output Formats

...

## 5. Exit Codes Summary

` ` `
+-------+--------------------------------------------------+
| Code  | Meaning                                          |
+-------+--------------------------------------------------+
| 0     | Success                                          |
| 1     | General error                                    |
+-------+--------------------------------------------------+
` ` `

---

## 6. Environment Variables

` ` `
+---------------------------+------------------------------+
| Variable                  | Effect                       |
+---------------------------+------------------------------+
| VAR_NAME                  | Description                  |
+---------------------------+------------------------------+
` ` `
```

---

## Quality Standards

1. **Completeness**: Every function, widget, action, constant, and data structure in
the source code must be documented. Do not omit anything.
2. **Precision**: Use exact values from the source code — pixel positions, color
codes, string literals, timing constants. Do not approximate.
3. **Traceability**: A developer should be able to rebuild the entire application
from these documents alone, without seeing the original source code.
4. **Cross-referencing**: Documents should reference each other where relevant. If
ACTIONS.md mentions a constant, reference the section in DATA_MODELS.md.
5. **Consistency**: Use the same terminology throughout all documents. If the source
code calls it `update_tree`, use `update_tree` everywhere.
