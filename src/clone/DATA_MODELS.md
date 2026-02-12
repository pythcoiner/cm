# GitSardine Constants & Configuration Specification

## 1. Configuration Schema

### 1.1 JSON Format

```
+----------------------------------------------------------+
| File: config.json                                        |
+----------------------------------------------------------+
| Technical Note: C++ uses JSON format via QJsonDocument.  |
| Python implementation used YAML (user.conf). JSON was    |
| chosen for C++ to avoid external dependencies and use    |
| Qt's built-in JSON support.                              |
|                                                          |
| Migration: None. C++ port is a standalone application.   |
| No automatic migration from Python user.conf file.       |
| Users configure fresh config.json on first run.          |
+----------------------------------------------------------+
{
  "paths": string[],      // Required, min 1 item
  "user": string,         // Required, non-empty
  "extend": number,       // Optional, default: 190
  "ignore": (string | string[])[]  // Optional, default: []
}
+----------------------------------------------------------+
```

### 1.2 Field Specifications

```
+----------------------------------------------------------+
| Field: paths                                             |
+----------------------------------------------------------+
| Type: Array of strings                                   |
| Required: Yes                                            |
| Description: Absolute paths to directories to scan       |
| Example: ["/home/user/projects", "/home/user/work"]      |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Field: user                                              |
+----------------------------------------------------------+
| Type: String                                             |
| Required: Yes                                            |
| Default: "user"                                          |
| Description: Username for branch naming validation       |
| Warning: If value is "user", show popup on startup       |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Field: extend                                            |
+----------------------------------------------------------+
| Type: Integer                                            |
| Required: No                                             |
| Default: 190                                             |
| Description: Pixels to add when window is extended       |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Field: ignore                                            |
+----------------------------------------------------------+
| Type: Array of (string | string[])                       |
| Required: No                                             |
| Default: []                                              |
| Description: Patterns to filter from changes list        |
|                                                          |
| Pattern matching:                                        |
| - String: matches if pattern is substring of filename    |
|   Example: ".tmp" matches "file.tmp", "foo.tmp.bak"      |
|                                                          |
| - Array of strings: ALL must match (AND condition)       |
|   Example: ["test", ".log"] matches "test_output.log"    |
|            but not "test.txt" or "output.log"            |
+----------------------------------------------------------+
```

### 1.3 Default Configuration

```json
{
  "paths": ["/home/path/"],
  "user": "user",
  "extend": 190,
  "ignore": []
}
```

---

## 2. Constants

### 2.1 Built-in Ignore Patterns

```
+----------------------------------------------------------+
| Patterns filtered from changes list (hardcoded)          |
+----------------------------------------------------------+
| ".idea/*"              | JetBrains IDE configuration     |
| "__pycache__/*"        | Python bytecode cache           |
| "venv/*"               | Python virtual environment      |
+----------------------------------------------------------+
```

### 2.2 Binary File Extensions (No Diff Display)

```
+----------------------------------------------------------+
| Extensions excluded from git diff tooltip                |
+----------------------------------------------------------+
| .ods      | OpenDocument Spreadsheet                     |
| .odg      | OpenDocument Graphics                        |
| .odt      | OpenDocument Text                            |
| .Z3PRT    | Solid Edge Part file                         |
| .Z3ASM    | Solid Edge Assembly file                     |
| .Z3DRW    | Solid Edge Drawing file                      |
| .exe      | Windows Executable                           |
| .stp      | STEP CAD file                                |
| .step     | STEP CAD file (alternate extension)          |
| .xrs      | Unknown binary                               |
| .pdf      | PDF document                                 |
+----------------------------------------------------------+
```

### 2.3 Text File Extensions (Diff Viewer Whitelist)

```
+----------------------------------------------------------+
| Extensions that enable the Diff Viewer Dialog            |
| (Right-click "Check diff" in changes tree)               |
+----------------------------------------------------------+

Source Code:
+---------------------------+------------------------------+
| .c, .h                    | C                            |
| .cpp, .hpp, .cc, .cxx     | C++                          |
| .py                       | Python                       |
| .js, .mjs, .cjs           | JavaScript                   |
| .ts, .tsx                 | TypeScript                   |
| .java                     | Java                         |
| .go                       | Go                           |
| .rs                       | Rust                         |
| .rb                       | Ruby                         |
| .php                      | PHP                          |
| .cs                       | C#                           |
| .swift                    | Swift                        |
| .kt, .kts                 | Kotlin                       |
| .lua                      | Lua                          |
| .pl, .pm                  | Perl                         |
| .r, .R                    | R                            |
| .sql                      | SQL                          |
+---------------------------+------------------------------+

Config/Data:
+---------------------------+------------------------------+
| .json                     | JSON                         |
| .yaml, .yml               | YAML                         |
| .xml                      | XML                          |
| .toml                     | TOML                         |
| .ini, .cfg                | INI/Config                   |
| .conf                     | Configuration                |
| .properties               | Java Properties              |
| .env                      | Environment variables        |
+---------------------------+------------------------------+

Markup/Documentation:
+---------------------------+------------------------------+
| .txt                      | Plain text                   |
| .md, .markdown            | Markdown                     |
| .rst                      | reStructuredText             |
| .html, .htm               | HTML                         |
| .css, .scss, .sass, .less | Stylesheets                  |
| .tex                      | LaTeX                        |
+---------------------------+------------------------------+

Shell/Scripts:
+---------------------------+------------------------------+
| .sh, .bash, .zsh          | Shell scripts                |
| .bat, .cmd, .ps1          | Windows scripts              |
+---------------------------+------------------------------+

Build/Project:
+---------------------------+------------------------------+
| .cmake                    | CMake                        |
| .make, Makefile           | Make                         |
| .gradle                   | Gradle                       |
| .pro                      | Qt Project                   |
+---------------------------+------------------------------+

Other:
+---------------------------+------------------------------+
| .gitignore                | Git ignore                   |
| .dockerignore             | Docker ignore                |
| .editorconfig             | Editor config                |
| Dockerfile                | Docker (no extension)        |
+---------------------------+------------------------------+

Fallback rule: If extension not in whitelist AND not in
binary blacklist (2.2), show diff viewer anyway (assume text).
```

### 2.4 Timing Constants

```
+----------------------------+---------------------------------+
| AUTO_UPDATE_INTERVAL       | 1800 seconds (30 minutes)       |
| CHANGES_UPDATE_INTERVAL    | 2000 ms (2 seconds)             |
| SPINNER_FRAME_DELAY        | 200 ms                          |
| PROGRESS_UPDATE_DELAY      | 100 ms                          |
| PROGRESS_STEP              | 5 (percentage points per tick)  |
+----------------------------+---------------------------------+
```

---

## 3. Icon Files

```
+----------------------------------------------------------+
| Required Icon Assets (icon/ directory)                   |
+----------------------------------------------------------+

Button Icons:
+---------------------------+------------------------------+
| arrow-circle-315.png      | update_tree, b_update        |
| cross-button.png          | b_delete (delete branch)     |
| arrow-skip-270.png        | b_pull                       |
| arrow-skip-090.png        | b_push                       |
| cross.png                 | b_delete_file                |
| arrow-curve-180-left.png  | b_clean (restore)            |
| disk--minus.png           | b_reset                      |
+---------------------------+------------------------------+

Navigation Icons:
+---------------------------+------------------------------+
| navigation-000-button-    | b_extend (expand right)      |
|  white.png                |                              |
| navigation-180-button-    | b_extend (collapse left)     |
|  white.png                |                              |
+---------------------------+------------------------------+

Spinner Animation (4 frames):
+---------------------------+------------------------------+
| spin/1.png                | Frame 1                      |
| spin/2.png                | Frame 2                      |
| spin/3.png                | Frame 3                      |
| spin/4.png                | Frame 4                      |
+---------------------------+------------------------------+

Repository Status Icons:
+---------------------------+------------------------------+
| exclamation-red.png       | Error state                  |
| drive-download.png        | Needs pull                   |
| drive-upload.png          | Needs push                   |
| disk--plus.png            | Needs commit (has changes)   |
| document.png              | Clean (up to date)           |
+---------------------------+------------------------------+

Tree Icons:
+---------------------------+------------------------------+
| folder-horizontal.png     | Folder in tree               |
| drive.png                 | Root drive/path              |
+---------------------------+------------------------------+

Application:
+---------------------------+------------------------------+
| gitsardine_icon.png           | Window icon (TBD - new icon  |
|                           | to be supplied later)        |
+---------------------------+------------------------------+
```

---

## 4. Status Messages

### 4.1 Success Messages

```
+-------------------------+--------------------------------+
| Operation               | Message                        |
+-------------------------+--------------------------------+
| File added              | "File added successfully"      |
| Commit success          | "Commit successful"            |
| Push success            | "Push successful"              |
| Pull success            | "{repo}:{branch} is up to date"|
| Merge success           | "Merge successful"             |
| Branch created          | "Branch created"               |
| Branch deleted          | "{branch} deleted"             |
| Reset success           | "Reset successful"             |
| Restore success         | "Restore successful"           |
+-------------------------+--------------------------------+
```

### 4.2 Error Messages

```
+-------------------------+--------------------------------+
| Condition               | Message                        |
+-------------------------+--------------------------------+
| Empty commit message    | "Commit message required"      |
| No files to commit      | "Nothing to commit"            |
| Uncommitted before pull | "Commit or discard changes     |
|                         |  before pull"                  |
| Pull failed             | "Pull failed"                  |
| Push failed             | "Push failed"                  |
| Merge failed            | "Merge failed"                 |
| Merge conflict          | "Merge conflict - resolve      |
|                         |  manually"                     |
| Branch change failed    | "Cannot switch branch"         |
| Delete master/main      | "Cannot delete master/main"    |
| Empty branch name       | "Branch name required"         |
| Branch exists           | "Branch already exists"        |
| No merge source         | "No branch to merge from"      |
| Add failed              | "Add failed"                   |
| Restore failed          | "Restore failed"               |
| Reset failed            | "Reset failed"                 |
| File not found          | "File not found"               |
| Cannot delete file      | "Cannot delete file"           |
| Cannot create branch    | "Cannot create branch"         |
+-------------------------+--------------------------------+
| Error tooltip format:                                    |
| "{command}\n     ==>    \n{stderr}"                      |
+----------------------------------------------------------+
```

### 4.3 Popup Dialogs

```
+----------------------------------------------------------+
| Dialog: User Not Defined                                 |
+----------------------------------------------------------+
| Title: "User Not Configured"                             |
| Text: "Please set your username in the config file."     |
| Trigger: config.user == "user"                           |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Dialog: Space in Branch Name                             |
+----------------------------------------------------------+
| Title: "Invalid Branch Name"                             |
| Text: "Spaces are not allowed in branch names.           |
|        Use '_' or '-' instead."                          |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Dialog: Empty Commit Message                             |
+----------------------------------------------------------+
| Title: "Commit Message Required"                         |
| Text: "Please enter a commit message."                   |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Dialog: Empty Branch Name                                |
+----------------------------------------------------------+
| Title: "Branch Name Required"                            |
| Text: "Please enter a branch name."                      |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Dialog: Username in Branch                               |
+----------------------------------------------------------+
| Title: "Branch Naming Convention"                        |
| Text: "Branch name should contain your username.         |
|        Example: {user}_feature or feature_{user}"        |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Dialog: Invalid Path (NEW)                               |
+----------------------------------------------------------+
| Title: "Invalid Path"                                    |
| Text: "The following path does not exist and will be     |
|        skipped: {path}"                                  |
| Trigger: Configured path not found during startup        |
+----------------------------------------------------------+
```

---

## 5. Status Priority

```
+----------------------------------------------------------+
| When multiple conditions are true, display priority:     |
+----------------------------------------------------------+
| 1. Error        (highest - show error icon)              |
| 2. Needs Pull                                            |
| 3. Needs Push                                            |
| 4. Needs Commit                                          |
| 5. Clean        (lowest - show clean icon)               |
+----------------------------------------------------------+
```
