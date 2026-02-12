# GitSardine Features Specification

## 1. Repository Discovery

### 1.1 Path Scanning

```
+----------------------------------------------------------+
| Function: list_projects()                                |
+----------------------------------------------------------+
| Purpose: Discover all git repositories in configured     |
|          directories                                     |
+----------------------------------------------------------+
| Algorithm:                                               |
| 1. For each path in config.paths:                        |
|    a. Walk directory tree recursively (os.walk)          |
|    b. Check if '.git' folder exists in subdirectories    |
|    c. If found, record as repository                     |
| 2. Build hierarchical project structure                  |
+----------------------------------------------------------+
| Output: Dictionary {project_path: [[repo_name, path]]}   |
+----------------------------------------------------------+
| Submodules: No special handling. Submodules appear as    |
| separate repositories if their .git folder is discovered.|
+----------------------------------------------------------+
```

### 1.2 Tree Building

```
+----------------------------------------------------------+
| Function: build_tree()                                   |
+----------------------------------------------------------+
| Purpose: Create visual tree structure from discovered    |
|          repositories                                    |
+----------------------------------------------------------+
| Algorithm:                                               |
| 1. Create root Folder item from first path segment       |
| 2. For each repository:                                  |
|    a. Get or create folder hierarchy                     |
|    b. Mark leaf as repository (is_repo=true)             |
|    c. Store original OS path (os_path)                   |
| 3. Attach model to QTreeView                             |
| 4. Sort alphabetically                                   |
| 5. Expand all nodes                                      |
+----------------------------------------------------------+
```

---

## 2. Repository Status Monitoring

### 2.1 Status Update (All Repositories)

```
+----------------------------------------------------------+
| Function: updates_repo_status()                          |
| Thread: Update (QThread)                                 |
+----------------------------------------------------------+
| Purpose: Check status of all discovered repositories     |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Disable update button                                 |
| 2. Mark all repos as "checking" (spinner icon)           |
| 3. For each repository:                                  |
|    a. Check pull status (fetch dry-run)                  |
|    b. Check push status (compare hashes)                 |
|    c. Check commit status (modified files)               |
|    d. Update icon based on status                        |
| 4. Re-enable update button                               |
+----------------------------------------------------------+
| Auto-trigger: Every 30 minutes (1800 seconds)            |
+----------------------------------------------------------+
```

### 2.2 Pull Status Check

```
+----------------------------------------------------------+
| Function: check_repo_status(repo)                        |
+----------------------------------------------------------+
| Command: git fetch -v --dry-run                          |
+----------------------------------------------------------+
| Logic:                                                   |
| 1. Execute fetch dry-run                                 |
| 2. Parse stderr output                                   |
|    - Normalize multiple spaces to single space           |
|      (regex: / +/ -> " ")                                |
|    - NOTE: Only needed for git CLI output parsing.       |
|      Not required when using libgit2 API.                |
| 3. Check for "origin/master" line:                       |
|    - If second field != "=" : needs_pull = true          |
|    - If second field == "=" : needs_pull = false         |
| 4. Empty output = error state                            |
+----------------------------------------------------------+
| Note: Only checks origin/master branch. Other branches   |
| are not monitored for pull availability.                 |
+----------------------------------------------------------+
```

### 2.3 Push Status Check

```
+----------------------------------------------------------+
| Function: check_push_status(repo)                        |
+----------------------------------------------------------+
| Method: Compare local and remote commit hashes           |
+----------------------------------------------------------+
| Files compared:                                          |
| - .git/refs/heads/master (local HEAD)                    |
| - .git/refs/remotes/origin/master (remote HEAD)          |
+----------------------------------------------------------+
| Logic:                                                   |
| 1. Read both files                                       |
| 2. If contents differ: needs_push = true                 |
| 3. If contents match: needs_push = false                 |
+----------------------------------------------------------+
| Note: Only compares master branch refs. Push status for  |
| other branches is not detected.                          |
+----------------------------------------------------------+
```

### 2.4 Commit Status Check

```
+----------------------------------------------------------+
| Function: check_commit_status(repo)                      |
+----------------------------------------------------------+
| Commands:                                                |
| - git ls-files -m -d -o -z --exclude-standard            |
| - git diff --name-only --cached                          |
+----------------------------------------------------------+
| Logic:                                                   |
| 1. Get modified/untracked files                          |
| 2. Get cached (staged) files                             |
| 3. If either list non-empty: needs_commit = true         |
+----------------------------------------------------------+
```

### 2.5 Single Repository Update

```
+----------------------------------------------------------+
| Function: check_single_status(path)                      |
| Thread: UpdateSingle (QThread)                           |
+----------------------------------------------------------+
| Purpose: Quick refresh of one repository after operation |
+----------------------------------------------------------+
| Trigger: After git operations (commit, push, pull, etc.) |
+----------------------------------------------------------+
```

---

## 3. Git Operations

### 3.1 Pull Operation

```
+----------------------------------------------------------+
| Function: do_pull()                                      |
| Thread: Bash (QThread)                                   |
+----------------------------------------------------------+
| Command: git pull origin {branch}                        |
+----------------------------------------------------------+
| Pre-conditions:                                          |
| 1. No uncommitted changes (change_list must be empty)    |
| 2. No other bash operation running                       |
| 3. Not locked (pull_lock = false)                        |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Check for uncommitted changes                         |
| 2. If changes exist: show error, abort                   |
| 3. Execute pull command in background thread             |
| 4. Show progress bar during operation                    |
| 5. On completion: update status, refresh branch info     |
+----------------------------------------------------------+
| Error handling:                                          |
| - Shows command + stderr in status tooltip               |
+----------------------------------------------------------+
```

### 3.2 Push Operation

```
+----------------------------------------------------------+
| Function: do_push()                                      |
| Thread: Bash (QThread)                                   |
+----------------------------------------------------------+
| Command: git push origin {branch}                        |
+----------------------------------------------------------+
| Pre-conditions:                                          |
| - No other bash operation running                        |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Execute push command in background thread             |
| 2. Show progress bar during operation                    |
| 3. On completion: update status                          |
+----------------------------------------------------------+
```

### 3.3 Commit Operation

```
+----------------------------------------------------------+
| Function: on_commit() / do_commit()                      |
+----------------------------------------------------------+
| Commands:                                                |
| - git add {file} (for each selected file)                |
| - git commit -m "{message}"                              |
+----------------------------------------------------------+
| Pre-conditions:                                          |
| 1. At least one file selected OR cached files exist      |
| 2. Commit message not empty                              |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Validate commit message exists                        |
| 2. For each checked file in tree:                        |
|    a. Handle filenames with spaces (quote appropriately) |
|    b. Execute git add                                    |
| 3. Execute git commit                                    |
| 4. Clear message input on success                        |
| 5. Refresh changes list                                  |
+----------------------------------------------------------+
| Filename handling:                                       |
| - Linux: 'filename with spaces'                          |
| - Windows: "filename with spaces"                        |
+----------------------------------------------------------+
```

### 3.4 Commit + Push Operation

```
+----------------------------------------------------------+
| Function: on_commit_push()                               |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Execute commit operation                              |
| 2. If commit succeeds: execute push operation            |
+----------------------------------------------------------+
```

### 3.5 Merge Operation

```
+----------------------------------------------------------+
| Function: do_merge(from_branch)                          |
+----------------------------------------------------------+
| Command: git merge {from_branch}                         |
+----------------------------------------------------------+
| Special behavior for master/main:                        |
| If current branch is master or main:                     |
|   Command: git merge {from} && git branch --delete {from}|
|   (Auto-deletes merged branch)                           |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Execute merge command                                 |
| 2. If on master/main: delete source branch               |
| 3. Update branch list                                    |
| 4. Refresh status                                        |
+----------------------------------------------------------+
| Conflict handling:                                       |
| If merge fails due to conflicts:                         |
| - Show error: "Merge conflict - resolve manually"        |
| - Tooltip shows git stderr output                        |
| - User must resolve conflicts externally (terminal/IDE)  |
| - No conflict resolution UI in this application          |
+----------------------------------------------------------+
```

### 3.6 Reset Operation

```
+----------------------------------------------------------+
| Function: do_reset()                                     |
+----------------------------------------------------------+
| Command: git reset                                       |
+----------------------------------------------------------+
| Purpose: Unstage all staged changes                      |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Execute reset command                                 |
| 2. Update branch info                                    |
| 3. Refresh status                                        |
+----------------------------------------------------------+
```

### 3.7 Restore Operation

```
+----------------------------------------------------------+
| Function: do_restore()                                   |
+----------------------------------------------------------+
| Command: git restore .                                   |
+----------------------------------------------------------+
| Purpose: Discard all working directory changes           |
+----------------------------------------------------------+
| Safety: Requires Ctrl key held (unlock_buttons)          |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Execute restore command                               |
| 2. Update branch info                                    |
| 3. Refresh status                                        |
+----------------------------------------------------------+
```

---

## 4. Branch Operations

### 4.1 Branch Listing

```
+----------------------------------------------------------+
| Functions: get_branches(), get_remotes(),                |
|            process_branches()                            |
+----------------------------------------------------------+
| Local branches:                                          |
| - Read from: .git/refs/heads/*                           |
| - Files in directory = branch names                      |
+----------------------------------------------------------+
| Remote branches:                                         |
| - Read from: .git/refs/remotes/{remote}/*                |
| - Displayed with angle brackets: <branch_name>           |
+----------------------------------------------------------+
| Final list order:                                        |
| 1. Current branch (selected_branch)                      |
| 2. Other local branches                                  |
| 3. Remote-only branches (in <>)                          |
| 4. "--new--" option                                      |
+----------------------------------------------------------+
| Detached HEAD:                                           |
| If repository is in detached HEAD state:                 |
| - Show error: "Detached HEAD - checkout a branch"        |
| - Disable git operations until branch is checked out     |
+----------------------------------------------------------+
```

### 4.2 Branch Switching

```
+----------------------------------------------------------+
| Function: on_branch_change(branch)                       |
+----------------------------------------------------------+
| Commands:                                                |
| - git stash                                              |
| - git checkout {branch}                                  |
| - git stash pop (if stash was created)                   |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Check if branch exists locally                        |
| 2. Stash any uncommitted changes (git stash)             |
|    - Note: git stash returns 0 even if nothing to stash  |
|    - Check stash list before/after to detect if created  |
| 3. If local: checkout directly                           |
| 4. If remote-only:                                       |
|    a. Create local branch                                |
|    b. Checkout                                           |
|    c. Pull from remote                                   |
| 5. Restore stashed changes (git stash pop)               |
|    - Only if stash was created in step 2                 |
|    - If pop fails (conflict), show error message         |
+----------------------------------------------------------+
| Stash conflict handling:                                 |
| If git stash pop fails due to conflicts:                 |
| - Show error: "Stash conflict - resolve manually"        |
| - Changes remain in stash (user must resolve)            |
+----------------------------------------------------------+
```

### 4.3 Branch Creation

```
+----------------------------------------------------------+
| Function: on_new_branch() / do_make_branch(name)         |
+----------------------------------------------------------+
| Command: git branch {name}                               |
+----------------------------------------------------------+
| Validation:                                              |
| 1. Name must not be empty                                |
| 2. Name must not contain spaces                          |
| 3. Name must contain username (from config)              |
| 4. Branch must not already exist                         |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Validate name                                         |
| 2. Execute git branch command                            |
| 3. Update branch list                                    |
| 4. Clear message input                                   |
+----------------------------------------------------------+
```

### 4.4 Branch Deletion

```
+----------------------------------------------------------+
| Function: do_delete_branch(branch)                       |
+----------------------------------------------------------+
| Command: git checkout master && git branch -D {branch}   |
+----------------------------------------------------------+
| Safety:                                                  |
| 1. Requires Ctrl key held                                |
| 2. Cannot delete master or main                          |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Checkout master first                                 |
| 2. Force delete branch (-D flag)                         |
| 3. Update branch list                                    |
+----------------------------------------------------------+
```

---

## 5. File Operations

### 5.1 View Changes

```
+----------------------------------------------------------+
| Function: check_changes(path)                            |
+----------------------------------------------------------+
| Command: git ls-files -m -d -o -z --exclude-standard     |
+----------------------------------------------------------+
| Output parsing:                                          |
| - Split by null character (\0)                           |
| - Filter out ignored patterns                            |
+----------------------------------------------------------+
| Built-in ignore patterns:                                |
| - .idea/* (JetBrains IDE files)                          |
| - __pycache__/* (Python cache)                           |
| - venv/* (Python virtual environment)                    |
| - Custom patterns from config.ignore                     |
+----------------------------------------------------------+
| Sorting: Alphabetical, case-insensitive                  |
+----------------------------------------------------------+
```

### 5.2 View Cached Changes

```
+----------------------------------------------------------+
| Function: check_cached_changes(path)                     |
+----------------------------------------------------------+
| Command: git diff --name-only --cached                   |
+----------------------------------------------------------+
| Purpose: List files staged for commit                    |
+----------------------------------------------------------+
| Sorting: Alphabetical, case-insensitive                  |
+----------------------------------------------------------+
```

### 5.3 View Diff

```
+----------------------------------------------------------+
| Function: do_git_diff(file)                              |
+----------------------------------------------------------+
| Command: git diff {file}                                 |
+----------------------------------------------------------+
| Binary file exclusions (no diff shown):                  |
| .ods, .odg, .odt, .Z3PRT, .Z3ASM, .exe,                  |
| .Z3DRW, .stp, .step, .xrs, .pdf                          |
+----------------------------------------------------------+
| Output: Displayed in file's tooltip in changes tree      |
+----------------------------------------------------------+
```

### 5.4 Add to Gitignore

```
+----------------------------------------------------------+
| Function: do_ignore(file)                                |
+----------------------------------------------------------+
| Purpose: Add file pattern to .gitignore                  |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Verify file exists                                    |
| 2. Append filename to .gitignore                         |
| 3. Refresh changes list                                  |
+----------------------------------------------------------+
```

### 5.5 Delete File

```
+----------------------------------------------------------+
| Function: do_delete_file(file)                           |
+----------------------------------------------------------+
| Purpose: Remove file from filesystem                     |
+----------------------------------------------------------+
| Safety: Requires Ctrl key held                           |
+----------------------------------------------------------+
| Process:                                                 |
| 1. Build full path                                       |
| 2. Remove file (os.remove)                               |
| 3. Refresh status                                        |
+----------------------------------------------------------+
```

---

## 6. UI State Management

### 6.1 Button Enable/Disable

```
+----------------------------------------------------------+
| Functions: disable_buttons(), enable_buttons()           |
+----------------------------------------------------------+
| Affected buttons (enable/disable):                       |
| - b_commit                                               |
| - b_commit_push                                          |
| - b_ignore                                               |
| - b_merge                                                |
| - b_pull                                                 |
| - b_push                                                 |
| - b_update                                               |
| - b_reset                                                |
+----------------------------------------------------------+
| Trigger: During git operations (disabled),               |
|          after completion (enabled)                      |
+----------------------------------------------------------+
```

### 6.2 Button Lock/Unlock (Destructive Operations)

```
+----------------------------------------------------------+
| Functions: lock_buttons(), unlock_buttons()              |
+----------------------------------------------------------+
| Affected buttons (lock/unlock):                          |
| - b_delete (delete branch)                               |
| - b_clean (restore)                                      |
| - b_delete_file                                          |
+----------------------------------------------------------+
| Unlock trigger: Ctrl key pressed                         |
| Lock trigger: Ctrl key released                          |
+----------------------------------------------------------+
```

### 6.3 Branch-Specific Visibility

```
+----------------------------------------------------------+
| Hidden on master/main branch:                            |
| - b_commit                                               |
| - b_commit_push                                          |
| - b_ignore                                               |
+----------------------------------------------------------+
| Purpose: Prevent direct commits to main branches         |
+----------------------------------------------------------+
```

### 6.4 Select All Changes

```
+----------------------------------------------------------+
| Functions: select_all_changes(), deselect_all_changes()  |
+----------------------------------------------------------+
| Trigger: "-- All --" checkbox state change               |
+----------------------------------------------------------+
| Behavior:                                                |
| - Checked: Check all file checkboxes                     |
| - Unchecked: Uncheck all file checkboxes                 |
+----------------------------------------------------------+
```

---

## 7. Progress Indication

### 7.1 Progress Bar

```
+----------------------------------------------------------+
| Thread: UpdateProgress (QThread)                         |
+----------------------------------------------------------+
| Behavior:                                                |
| - Hidden by default                                      |
| - Shown during long operations (pull, push)              |
| - Loops from 0 to 100 continuously                       |
| - Update interval: 100ms                                 |
+----------------------------------------------------------+
```

### 7.2 Spinner Animation

```
+----------------------------------------------------------+
| Thread: Spin (QThread)                                   |
+----------------------------------------------------------+
| Behavior:                                                |
| - Cycles through spin/1.png to spin/4.png                |
| - Update interval: 200ms                                 |
| - Active during repository status update                 |
+----------------------------------------------------------+
```

---

## 8. Configuration

### 8.1 Config File Format (JSON)

```
+----------------------------------------------------------+
| File: config.json                                        |
+----------------------------------------------------------+
| Fields:                                                  |
+----------------------------------------------------------+
| paths: string[]                                          |
|   - List of root directories to scan                     |
|   - Example: ["/home/user/projects", "/home/user/work"]  |
+----------------------------------------------------------+
| user: string                                             |
|   - Username for branch naming validation                |
|   - New branches must contain this string                |
|   - Example: "john"                                      |
+----------------------------------------------------------+
| extend: int                                              |
|   - Pixels to add when window is extended                |
|   - Default: 190                                         |
+----------------------------------------------------------+
| ignore: string[] | string[][]                            |
|   - Additional patterns to ignore in changes list        |
|   - Single string: match anywhere in filename            |
|   - Array of strings: all must match (AND condition)     |
|   - Example: ["*.tmp", ".DS_Store", ["test", ".log"]]    |
+----------------------------------------------------------+
```

### 8.2 Default Config Creation

```
+----------------------------------------------------------+
| Trigger: Config file does not exist                      |
+----------------------------------------------------------+
| Default content:                                         |
| {                                                        |
|   "paths": ["/home/path/"],                              |
|   "user": "user",                                        |
|   "extend": 190,                                         |
|   "ignore": []                                           |
| }                                                        |
+----------------------------------------------------------+
| Warning: Popup shown if user == "user"                   |
+----------------------------------------------------------+
```

---

## 9. Status Messages

### 9.1 Status Label Updates

```
+----------------------------------------------------------+
| Function: set_label(txt, tooltip=None)                   |
+----------------------------------------------------------+
| Display: Main text in label, details in tooltip          |
+----------------------------------------------------------+
| Common messages:                                         |
+----------------------------------------------------------+
| Success:                                                 |
| - "commit done successfull"                              |
| - "push done successfull"                                |
| - "{repo} : {branch} is up to date"                      |
| - "merged successfully"                                  |
| - "branch created"                                       |
| - "{branch} deleted"                                     |
| - "reset successfully"                                   |
+----------------------------------------------------------+
| Errors:                                                  |
| - "commit message is needed!"                            |
| - "nothing to commit!"                                   |
| - "commit or delete before pull!"                        |
| - "Cannot pull!"                                         |
| - "push fail!"                                           |
| - "merge fail!"                                          |
| - "Cannot change branch"                                 |
| - "cannot delete master/main!"                           |
| - "need define a branch name!"                           |
| - "branch already exist"                                 |
| - "no branch to merge from!"                             |
+----------------------------------------------------------+
| Tooltip format for errors:                               |
| "{command}\n     ==>    \n{stderr}"                      |
+----------------------------------------------------------+
```

---

## 10. Threading Model

### 10.1 Thread Classes

```
+----------------------------------------------------------+
| Bash (QThread)                                           |
+----------------------------------------------------------+
| Purpose: Execute git commands asynchronously             |
| Signals:                                                 |
| - strt: Emitted when starting                            |
| - ret(object): Emitted with subprocess result            |
| Used for: Pull, Push operations                          |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Update (QThread)                                         |
+----------------------------------------------------------+
| Purpose: Refresh all repository statuses                 |
| No signals (modifies UI directly via parent reference)   |
| Disables update_tree button during execution             |
+----------------------------------------------------------+

+----------------------------------------------------------+
| UpdateSingle (QThread)                                   |
+----------------------------------------------------------+
| Purpose: Quick refresh of one repository                 |
| Property: path - repository to update                    |
| Used after: Any git operation on current repo            |
+----------------------------------------------------------+

+----------------------------------------------------------+
| UpdateProgress (QThread)                                 |
+----------------------------------------------------------+
| Purpose: Animate progress bar                            |
| Signals:                                                 |
| - update_progress(int): Progress value (0-100)           |
| - ended: Animation complete                              |
| Loop interval: 100ms                                     |
+----------------------------------------------------------+

+----------------------------------------------------------+
| Spin (QThread)                                           |
+----------------------------------------------------------+
| Purpose: Delay timer with completion signal              |
| Parameter: time - delay in seconds                       |
| Signal: ended - emitted after delay                      |
| Used for: Icon animation, auto-update timers             |
+----------------------------------------------------------+

+----------------------------------------------------------+
| ChangesUpdateWorker (QThread)                            |
+----------------------------------------------------------+
| Purpose: Auto-refresh changes list                       |
| Interval: 2 seconds                                      |
| Status: ENABLED (runs continuously when repo selected)   |
| Trigger: Automatically started when repository selected  |
+----------------------------------------------------------+
```

---

## 11. Context Menus

### 11.1 Changes Tree Context Menu

```
+----------------------------------------------------------+
| Trigger: Right-click on changes tree                     |
+----------------------------------------------------------+
| Menu items:                                              |
| - "Check diff" - Opens diff viewer dialog for file       |
+----------------------------------------------------------+
| Position: At click location (mapToGlobal)                |
+----------------------------------------------------------+
| Diff Viewer:                                             |
| - Opens modal dialog with syntax-highlighted diff        |
| - Only available for whitelisted text file extensions    |
| - Whitelist defined in DATA_MODELS.md section 2.3        |
+----------------------------------------------------------+
```

### 11.2 Folder Tree Context Menu (PHASE 2)

```
+----------------------------------------------------------+
| IMPLEMENTATION PHASE: 2 (not in initial release)         |
+----------------------------------------------------------+
| Trigger: Right-click on folder tree item                 |
+----------------------------------------------------------+
| Policy: CustomContextMenu                                |
+----------------------------------------------------------+
| Menu items (repository items only):                      |
| - "Open in file manager" - Opens repo folder             |
| - "Open in terminal" - Opens terminal at repo path       |
| - "Copy path" - Copies repo path to clipboard            |
| - "Refresh status" - Updates status for this repo        |
+----------------------------------------------------------+
| Note: New feature in C++ (not implemented in Python)     |
| Execution chains to be defined in Phase 2 specification. |
+----------------------------------------------------------+
```
