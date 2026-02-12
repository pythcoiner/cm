# GitSardine User Actions Specification

## 1. Button Actions

### 1.1 Update Repositories Button (`update_tree`)

```
+------------------------------------------------------------------+
| Widget: update_tree (QPushButton)                                |
| Event: clicked                                                   |
| Connection: self.ui.update_tree.clicked.connect(                 |
|             self.updates_repo_status)                            |
+------------------------------------------------------------------+
| Function Called: updates_repo_status()                           |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. updates_repo_status()                                         |
|    |                                                             |
|    +-> status_update.start()     [Update QThread]                |
|    +-> spin.start()              [Spin QThread - icon animation] |
|                                                                  |
| 2. Update.run() [in thread]                                      |
|    |                                                             |
|    +-> ui.update_tree.setEnabled(False)                          |
|    +-> list_items()              [get all tree items]            |
|    +-> FOR each repo:                                            |
|        +-> item.status_checked = False                           |
|        +-> item.update_icon()    [show spinner]                  |
|    +-> FOR each repo:                                            |
|        +-> check_repo_status(repo)                               |
|            +-> git fetch -v --dry-run                            |
|            +-> check_push_status(repo)                           |
|            +-> check_commit_status(repo)                         |
|    +-> ui.update_tree.setEnabled(True)                           |
|                                                                  |
| 3. Spin.run() [parallel thread]                                  |
|    |                                                             |
|    +-> sleep(0.2)                                                |
|    +-> ended.emit()                                              |
|                                                                  |
| 4. update_spin() [on Spin.ended signal]                          |
|    |                                                             |
|    +-> IF status_update.is_running:                              |
|        +-> cycle spin icon (1->2->3->4->1)                       |
|        +-> spin.start() [restart animation]                      |
|    +-> ELSE:                                                     |
|        +-> reset icon to spin/1.png                              |
+------------------------------------------------------------------+
```

### 1.2 Extend Button (`b_extend`)

```
+------------------------------------------------------------------+
| Widget: b_extend (QPushButton)                                   |
| Event: clicked                                                   |
| Connection: self.ui.b_extend.clicked.connect(self.on_b_extend)   |
+------------------------------------------------------------------+
| Function Called: on_b_extend()                                   |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| IF NOT is_extended:                                              |
|    +-> is_extended = True                                        |
|    +-> b_extend.setIcon(left_arrow_icon)                         |
|    +-> ui.setFixedWidth(ui.width() + extend)                     |
|    +-> self.setFixedWidth(self.width() + extend)                 |
|    +-> folder_tree.setFixedWidth(folder_tree.width() + extend)   |
|    +-> tree.setFixedWidth(tree.width() + extend)                 |
|    +-> b_extend.move(b_extend.x() + extend, b_extend.y())        |
|                                                                  |
| ELSE:                                                            |
|    +-> is_extended = False                                       |
|    +-> b_extend.setIcon(right_arrow_icon)                        |
|    +-> ui.setFixedWidth(ui.width() - extend)                     |
|    +-> self.setFixedWidth(self.width() - extend)                 |
|    +-> folder_tree.setFixedWidth(folder_tree.width() - extend)   |
|    +-> tree.setFixedWidth(tree.width() - extend)                 |
|    +-> b_extend.move(b_extend.x() - extend, b_extend.y())        |
+------------------------------------------------------------------+
```

### 1.3 Delete Branch Button (`b_delete`)

```
+------------------------------------------------------------------+
| Widget: b_delete (QPushButton)                                   |
| Event: clicked                                                   |
| Connection: self.ui.b_delete.clicked.connect(                    |
|             self.on_delete_branch)                               |
| Precondition: Ctrl key must be held (button disabled by default) |
+------------------------------------------------------------------+
| Function Called: on_delete_branch()                              |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_delete_branch()                                            |
|    |                                                             |
|    +-> branch = combo_branch.currentText()                       |
|    +-> do_delete_branch(branch)                                  |
|                                                                  |
| 2. do_delete_branch(branch)                                      |
|    |                                                             |
|    +-> IF branch == "master" OR branch == "main":                |
|    |   +-> set_label("Cannot delete master/main")               |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> ELSE:                                                     |
|        +-> cmd = "git checkout master && git branch -D {branch}" |
|        +-> subprocess.run(cmd)                                   |
|        +-> IF returncode != 0:                                   |
|        |   +-> set_label("Cannot delete branch!", tooltip)       |
|        |   +-> RETURN Result::Err(error)                         |
|        +-> ELSE:                                                 |
|            +-> set_label("{branch} deleted", tooltip)            |
|            +-> update_branch()                                   |
|            +-> RETURN Result::Ok()                                   |
+------------------------------------------------------------------+
```

### 1.4 Update Changes Button (`b_update`)

```
+------------------------------------------------------------------+
| Widget: b_update (QPushButton)                                   |
| Event: clicked                                                   |
| Connection: self.ui.b_update.clicked.connect(self.on_update)     |
+------------------------------------------------------------------+
| Function Called: on_update()                                     |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_update()                                                   |
|    |                                                             |
|    +-> check_changes()                                           |
|    +-> update_changes()                                          |
|                                                                  |
| 2. check_changes()                                               |
|    |                                                             |
|    +-> cmd = "git ls-files -m -d -o -z --exclude-standard"       |
|    +-> subprocess.run(cmd)                                       |
|    +-> Parse output, filter ignored patterns                     |
|    +-> self.change_list = filtered_files                         |
|    +-> check_cached_changes()                                    |
|        +-> cmd = "git diff --name-only --cached"                 |
|        +-> self.cached_change_list = result                      |
|    +-> update_changes()                                          |
|                                                                  |
| 3. update_changes()                                              |
|    |                                                             |
|    +-> tree.clear()                                              |
|    +-> IF change_list not empty:                                 |
|    |   +-> Add header "---- Modified files ----"                 |
|    |   +-> Add "-- All --" checkbox item                         |
|    |   +-> FOR each file in change_list:                         |
|    |       +-> Create QTreeWidgetItem with checkbox              |
|    |       +-> Set tooltip with diff (do_git_diff)               |
|    +-> IF cached_change_list not empty:                          |
|        +-> Add header "---- Cached files ----"                   |
|        +-> FOR each file in cached_change_list:                  |
|            +-> Create QTreeWidgetItem (no checkbox)              |
+------------------------------------------------------------------+
```

### 1.5 Pull Button (`b_pull`)

```
+------------------------------------------------------------------+
| Widget: b_pull (QPushButton)                                     |
| Event: clicked                                                   |
| Connection: self.ui.b_pull.clicked.connect(self.on_pull)         |
+------------------------------------------------------------------+
| Function Called: on_pull() -> do_pull()                          |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_pull()                                                     |
|    |                                                             |
|    +-> do_pull()                                                 |
|                                                                  |
| 2. do_pull()                                                     |
|    |                                                             |
|    +-> IF pull_lock OR bash.is_running:                          |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> pull_lock = True                                          |
|    +-> get_selected_branch()                                     |
|    +-> check_changes()                                           |
|    |                                                             |
|    +-> IF change_list NOT empty:                                 |
|    |   +-> set_label("Commit or discard changes before pull")            |
|    |   +-> pull_lock = False                                     |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> set_label("Start pull on branch: {branch}")               |
|    +-> cmd = "git pull origin {branch}"                          |
|    +-> bash_action = 'do_pull'                                   |
|    +-> bash.cmd = cmd                                            |
|    +-> bash.start()              [Bash QThread]                  |
|    +-> start_progress()                                          |
|                                                                  |
| 3. Bash.run() [in thread]                                        |
|    |                                                             |
|    +-> disable_buttons()                                         |
|    +-> progress.start()                                          |
|    +-> subprocess.run(cmd)                                       |
|    +-> ret.emit(result)                                          |
|                                                                  |
| 4. bash_ret(ret) [on Bash.ret signal]                            |
|    |                                                             |
|    +-> IF bash_action == 'do_pull':                              |
|        +-> ret_pull(ret)                                         |
|                                                                  |
| 5. ret_pull(ret)                                                 |
|    |                                                             |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Pull failed", tooltip)                    |
|    +-> ELSE:                                                     |
|    |   +-> set_label("{repo}:{branch} is up to date", tooltip)   |
|    +-> update_branch(label=False)                                |
|    +-> bash.cmd = ''                                             |
|    +-> bash_action = None                                        |
|    +-> check_single_status(path)                                 |
|    +-> pull_lock = False                                         |
+------------------------------------------------------------------+
```

### 1.6 Push Button (`b_push`)

```
+------------------------------------------------------------------+
| Widget: b_push (QPushButton)                                     |
| Event: clicked                                                   |
| Connection: self.ui.b_push.clicked.connect(self.on_push)         |
+------------------------------------------------------------------+
| Function Called: on_push() -> do_push()                          |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_push()                                                     |
|    |                                                             |
|    +-> do_push()                                                 |
|                                                                  |
| 2. do_push()                                                     |
|    |                                                             |
|    +-> IF bash.is_running:                                       |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> cmd = "git push origin {branch}"                          |
|    +-> bash_action = 'do_push'                                   |
|    +-> bash.cmd = cmd                                            |
|    +-> bash.start()              [Bash QThread]                  |
|    +-> start_progress()                                          |
|                                                                  |
| 3. Bash.run() [in thread]                                        |
|    |                                                             |
|    +-> disable_buttons()                                         |
|    +-> progress.start()                                          |
|    +-> subprocess.run(cmd)                                       |
|    +-> ret.emit(result)                                          |
|                                                                  |
| 4. bash_ret(ret) [on Bash.ret signal]                            |
|    |                                                             |
|    +-> IF bash_action == 'do_push':                              |
|        +-> ret_push(ret)                                         |
|                                                                  |
| 5. ret_push(ret)                                                 |
|    |                                                             |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Push failed", tooltip)                      |
|    +-> ELSE:                                                     |
|    |   +-> set_label("Push successful", tooltip)           |
|    +-> bash.cmd = ''                                             |
|    +-> bash_action = None                                        |
|    +-> check_single_status(path)                                 |
+------------------------------------------------------------------+
```

### 1.7 Delete File Button (`b_delete_file`)

```
+------------------------------------------------------------------+
| Widget: b_delete_file (QPushButton)                              |
| Event: clicked                                                   |
| Connection: self.ui.b_delete_file.clicked.connect(               |
|             self.on_delete_file)                                 |
| Precondition: Ctrl key must be held (button disabled by default) |
+------------------------------------------------------------------+
| Function Called: on_delete_file()                                |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_delete_file()                                              |
|    |                                                             |
|    +-> FOR each item in tree_list:                               |
|        +-> IF item.checkState(0) == Checked:                     |
|            +-> filename = item.text(0)                           |
|            +-> do_delete_file(filename)                          |
|    +-> check_changes()                                           |
|                                                                  |
| 2. do_delete_file(file) -> Result<void, QString>                 |
|    |                                                             |
|    +-> path = self.path + slash + file                           |
|    +-> IF !QFile::exists(path):                                  |
|    |   +-> RETURN Result::Err("File not found")                  |
|    +-> IF !QFile::remove(path):                                  |
|    |   +-> set_label("Cannot delete file")                       |
|    |   +-> check_single_status(self.path)                        |
|    |   +-> RETURN Result::Err("Cannot delete file")              |
|    +-> check_single_status(self.path)                            |
|    +-> RETURN Result::Ok()                                       |
+------------------------------------------------------------------+
```

### 1.8 Restore Button (`b_clean`)

```
+------------------------------------------------------------------+
| Widget: b_clean (QPushButton)                                    |
| Event: clicked                                                   |
| Connection: self.ui.b_clean.clicked.connect(self.do_restore)     |
| Precondition: Ctrl key must be held (button disabled by default) |
+------------------------------------------------------------------+
| Function Called: do_restore()                                    |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. do_restore()                                                  |
|    |                                                             |
|    +-> cmd = "git restore ."                                     |
|    +-> subprocess.run(cmd)                                       |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Restore failed", tooltip)                    |
|    |   +-> update_branch()                                       |
|    |   +-> check_single_status(path)                             |
|    |   +-> RETURN False                                          |
|    +-> ELSE:                                                     |
|        +-> set_label("Restore successful", tooltip)             |
|        +-> update_branch()                                       |
|        +-> check_single_status(path)                             |
|        +-> RETURN True                                           |
+------------------------------------------------------------------+
```

### 1.9 Reset Button (`b_reset`)

```
+------------------------------------------------------------------+
| Widget: b_reset (QPushButton)                                    |
| Event: clicked                                                   |
| Connection: self.ui.b_reset.clicked.connect(self.do_reset)       |
+------------------------------------------------------------------+
| Function Called: do_reset()                                      |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. do_reset()                                                    |
|    |                                                             |
|    +-> cmd = "git reset"                                         |
|    +-> subprocess.run(cmd)                                       |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Reset failed", tooltip)                     |
|    |   +-> update_branch()                                       |
|    |   +-> check_single_status(path)                             |
|    |   +-> RETURN False                                          |
|    +-> ELSE:                                                     |
|        +-> set_label("Reset successful", tooltip)              |
|        +-> update_branch()                                       |
|        +-> check_single_status(path)                             |
|        +-> RETURN True                                           |
+------------------------------------------------------------------+
```

### 1.10 Commit Button (`b_commit`)

```
+------------------------------------------------------------------+
| Widget: b_commit (QPushButton)                                   |
| Event: clicked                                                   |
| Connection: self.ui.b_commit.clicked.connect(self.on_commit)     |
| Visibility: Hidden when branch is master or main                 |
+------------------------------------------------------------------+
| Function Called: on_commit()                                     |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_commit()                                                   |
|    |                                                             |
|    +-> IF cached_change_list empty AND change_list empty:        |
|    |   +-> set_label("Nothing to commit")                       |
|    |   +-> RETURN False                                          |
|    |                                                             |
|    +-> IF msg.text() == '':                                      |
|    |   +-> set_label("Commit message required")                |
|    |   +-> RETURN False                                          |
|    |                                                             |
|    +-> FOR each item in tree_list:                               |
|    |   +-> IF item.checkState == Checked AND item != "-- All --":|
|    |       +-> filename = item.text(0)                           |
|    |       +-> IF ' ' in filename:                               |
|    |       |   +-> Linux: filename = '{filename}'                |
|    |       |   +-> Windows: filename = "{filename}"              |
|    |       +-> IF NOT do_add(filename):                          |
|    |           +-> RETURN False                                  |
|    |                                                             |
|    +-> IF NOT do_commit():                                       |
|    |   +-> check_changes()                                       |
|    |   +-> RETURN False                                          |
|    |                                                             |
|    +-> check_changes()                                           |
|    +-> RETURN True                                               |
|                                                                  |
| 2. do_add(file)                                                  |
|    |                                                             |
|    +-> cmd = "git add {file}"                                    |
|    +-> subprocess.run(cmd)                                       |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Add failed", tooltip)                       |
|    |   +-> RETURN False                                          |
|    +-> ELSE:                                                     |
|        +-> set_label("File added successfully", tooltip)           |
|        +-> RETURN True                                           |
|                                                                  |
| 3. do_commit()                                                   |
|    |                                                             |
|    +-> msg = ui.msg.text()                                       |
|    +-> IF msg == '':                                             |
|    |   +-> set_label("Commit message required")          |
|    |   +-> RETURN False                                          |
|    +-> cmd = 'git commit -m "{msg}"'                             |
|    +-> subprocess.run(cmd)                                       |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Commit failed", tooltip)                    |
|    |   +-> check_single_status(path)                             |
|    |   +-> RETURN False                                          |
|    +-> ELSE:                                                     |
|        +-> set_label("Commit successful", tooltip)         |
|        +-> ui.msg.setText('')                                    |
|        +-> check_single_status(path)                             |
|        +-> RETURN True                                           |
+------------------------------------------------------------------+
```

### 1.11 Commit+Push Button (`b_commit_push`)

```
+------------------------------------------------------------------+
| Widget: b_commit_push (QPushButton)                              |
| Event: clicked                                                   |
| Connection: self.ui.b_commit_push.clicked.connect(               |
|             self.on_commit_push)                                 |
| Visibility: Hidden when branch is master or main                 |
+------------------------------------------------------------------+
| Function Called: on_commit_push()                                |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. on_commit_push()                                              |
|    |                                                             |
|    +-> IF NOT on_commit():                                       |
|    |   +-> RETURN False                                          |
|    |                                                             |
|    +-> do_push()                                                 |
|                                                                  |
| (See on_commit and do_push execution chains above)               |
+------------------------------------------------------------------+
```

### 1.12 Ignore Button (`b_ignore`)

```
+------------------------------------------------------------------+
| Widget: b_ignore (QPushButton)                                   |
| Event: clicked                                                   |
| Connection: self.ui.b_ignore.clicked.connect(self.on_ignore)     |
| Visibility: Hidden when branch is master or main                 |
+------------------------------------------------------------------+
| Function Called: on_ignore()                                     |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_ignore()                                                   |
|    |                                                             |
|    +-> FOR each item in tree_list:                               |
|        +-> IF item.checkState(0) == Checked:                     |
|            +-> filename = item.text(0)                           |
|            +-> IF ' ' in filename:                               |
|            |   +-> filename = '{filename}'                       |
|            +-> do_ignore(filename)                               |
|    +-> check_changes()                                           |
|                                                                  |
| 2. do_ignore(file)                                               |
|    |                                                             |
|    +-> Strip quotes if present                                   |
|    +-> path = self.path + slash + file                           |
|    +-> gitignore_path = self.path + slash + '.gitignore'         |
|    +-> IF NOT os.path.exists(path):                              |
|    |   +-> set_label("File not found")                        |
|    |   +-> RETURN False                                          |
|    +-> Open gitignore_path in append mode                        |
|    +-> Write file + '\n'                                         |
|    +-> Close file                                                |
|    +-> RETURN True                                               |
+------------------------------------------------------------------+
```

### 1.13 Merge Button (`b_merge`)

```
+------------------------------------------------------------------+
| Widget: b_merge (QPushButton)                                    |
| Event: clicked                                                   |
| Connection: self.ui.b_merge.clicked.connect(self.on_merge)       |
+------------------------------------------------------------------+
| Function Called: on_merge()                                      |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_merge()                                                    |
|    |                                                             |
|    +-> _from = combo_merge.currentText()                         |
|    +-> IF _from != '':                                           |
|    |   +-> do_merge(_from)                                       |
|    +-> ELSE:                                                     |
|        +-> set_label("No branch to merge from")                 |
|                                                                  |
| 2. do_merge(_from)                                               |
|    |                                                             |
|    +-> branch = combo_branch.currentText()                       |
|    +-> cmd = "git merge {_from}"                                 |
|    +-> IF branch == "master" OR branch == "main":                |
|    |   +-> cmd += " && git branch --delete {_from}"              |
|    +-> subprocess.run(cmd)                                       |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Merge failed", tooltip)                     |
|    |   +-> check_single_status(path)                             |
|    |   +-> RETURN False                                          |
|    +-> ELSE:                                                     |
|        +-> set_label("Merge successful", tooltip)             |
|        +-> update_branch()                                       |
|        +-> check_single_status(path)                             |
|        +-> RETURN True                                           |
+------------------------------------------------------------------+
```

---

## 2. Selection Actions

### 2.1 Repository Double-Click (`folder_tree`)

```
+------------------------------------------------------------------+
| Widget: folder_tree (QTreeView)                                  |
| Event: doubleClicked                                             |
| Connection: self.ui.folder_tree.doubleClicked.connect(           |
|             self.on_repo_selected)                               |
+------------------------------------------------------------------+
| Function Called: on_repo_selected(index)                         |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_repo_selected(index)                                       |
|    |                                                             |
|    +-> item = model.itemFromIndex(index)                         |
|    +-> IF NOT item.is_repo:                                      |
|    |   +-> [Toggle expand/collapse of folder]                    |
|    |   +-> IF folder_tree.isExpanded(index):                     |
|    |   |   +-> folder_tree.collapse(index)                       |
|    |   +-> ELSE:                                                 |
|    |   |   +-> folder_tree.expand(index)                         |
|    |   +-> RETURN                                                |
|    +-> self.path = item.os_path                                  |
|    +-> section = path.split(slash)[-1]                           |
|    +-> self.section = [section, path]                            |
|    +-> update_branch()                                           |
|    +-> repo_selected = True                                      |
|                                                                  |
| 2. update_branch(label=True)                                     |
|    |                                                             |
|    +-> IF update_brch_lock:                                      |
|    |   +-> RETURN                                                |
|    +-> update_brch_lock = True                                   |
|    +-> get_branches()                                            |
|    |   +-> Read .git/refs/heads/*                                |
|    |   +-> self.branches = file list                             |
|    |   +-> self.local_branches = file list                       |
|    +-> get_remotes()                                             |
|    |   +-> Read .git/refs/remotes/{remote}/*                     |
|    |   +-> self.remotes = {remote: [branches]}                   |
|    +-> get_selected_branch()                                     |
|    |   +-> Read .git/HEAD                                        |
|    |   +-> self.selected_branch = branch name                    |
|    +-> process_branches()                                        |
|    |   +-> Combine local + remote branches                       |
|    |   +-> Mark remote-only with < >                             |
|    +-> check_changes()                                           |
|    +-> combo_branch.clear()                                      |
|    +-> combo_branch.addItems(branches + ['--new--'])             |
|    +-> combo_merge.clear()                                       |
|    +-> combo_merge.addItems(branches without selected)           |
|    +-> IF label:                                                 |
|    |   +-> set_label("{repo} : {branch}")                        |
|    +-> label_repo.setText("{repo} : {branch}")                   |
|    +-> update_brch_lock = False                                  |
|    +-> enable_buttons()                                          |
+------------------------------------------------------------------+
```

### 2.2 Branch Selection (`combo_branch`)

```
+------------------------------------------------------------------+
| Widget: combo_branch (QComboBox)                                 |
| Event: currentTextChanged                                        |
| Connection: self.ui.combo_branch.currentTextChanged.connect(     |
|             self.on_branch_choice)                               |
+------------------------------------------------------------------+
| Function Called: on_branch_choice()                              |
+------------------------------------------------------------------+
| Execution Chain:                                                 |
|                                                                  |
| 1. on_branch_choice()                                            |
|    |                                                             |
|    +-> branch = combo_branch.currentText()                       |
|    |                                                             |
|    +-> IF branch == "master" OR branch == "main":                |
|    |   +-> b_commit.setVisible(False)                            |
|    |   +-> b_commit_push.setVisible(False)                       |
|    |   +-> b_ignore.setVisible(False)                            |
|    +-> ELSE:                                                     |
|    |   +-> b_commit.setVisible(True)                             |
|    |   +-> b_commit_push.setVisible(True)                        |
|    |   +-> b_ignore.setVisible(True)                             |
|    |                                                             |
|    +-> IF branch == '':                                          |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> IF branch != '--new--':                                   |
|    |   +-> IF branch starts with '<' AND ends with '>':          |
|    |   |   +-> branch = branch[1:-1]  (strip angle brackets)     |
|    |   +-> on_branch_change(branch)                              |
|    +-> ELSE:                                                     |
|        +-> on_new_branch()                                       |
|                                                                  |
| 2a. on_branch_change(branch) [existing branch]                   |
|    |                                                             |
|    +-> IF branch_chg_lock:                                       |
|    |   +-> RETURN                                                |
|    +-> branch_chg_lock = True                                    |
|    +-> IF branch is None OR len(branch) == 0:                    |
|    |   +-> branch_chg_lock = False                               |
|    |   +-> RETURN                                                |
|    +-> get_selected_branch()                                     |
|    +-> get_branches()                                            |
|    +-> stash_created = do_stash()  [stash uncommitted changes]   |
|    +-> IF branch in local_branches:                              |
|    |   +-> do_checkout(branch)                                   |
|    +-> ELSE:                                                     |
|    |   +-> do_make_branch(branch)                                |
|    |   +-> do_checkout(branch)                                   |
|    |   +-> do_pull()                                             |
|    +-> IF stash_created:                                         |
|    |   +-> do_stash_pop()  [restore stashed changes]             |
|    +-> branch_chg_lock = False                                   |
|                                                                  |
| 2b. on_new_branch() [--new-- selected]                           |
|    |                                                             |
|    +-> name = msg.text()                                         |
|    +-> IF name == '':                                            |
|    |   +-> set_label("Branch name required")                     |
|    |   +-> popup_enter_branch_name()                             |
|    |   +-> RETURN                                                |
|    +-> IF ' ' in name:                                           |
|    |   +-> popup_space_in_branch_name()                          |
|    |   +-> RETURN                                                |
|    +-> IF self.user NOT in name:                                 |
|    |   +-> popup_username_in_branch()                            |
|    |   +-> RETURN                                                |
|    +-> IF name NOT in branches:                                  |
|    |   +-> do_make_branch(name)                                  |
|    |   +-> msg.clear()                                           |
|    +-> ELSE:                                                     |
|        +-> set_label("Branch already exists")                    |
|                                                                  |
| 3. do_stash() -> bool                                            |
|    |                                                             |
|    +-> stash_count_before = git stash list | wc -l               |
|    +-> cmd = "git stash"                                         |
|    +-> subprocess.run(cmd)                                       |
|    +-> stash_count_after = git stash list | wc -l                |
|    +-> RETURN (stash_count_after > stash_count_before)           |
|    |                                                             |
|    NOTE: C++ improvement. Python used auto-commit instead of     |
|    stash, which polluted commit history. C++ uses proper stash.  |
|                                                                  |
| 3b. do_stash_pop()                                               |
|    |                                                             |
|    +-> cmd = "git stash pop"                                     |
|    +-> subprocess.run(cmd)                                       |
|    +-> IF returncode != 0:                                       |
|    |   +-> set_label("Stash conflict - resolve manually",        |
|    |       tooltip)                                              |
|    |   +-> [Changes remain in stash for manual resolution]       |
|    +-> ELSE:                                                     |
|        +-> [Changes restored successfully]                       |
|                                                                  |
| 4. do_checkout(branch)                                           |
|    |                                                             |
|    +-> get_selected_branch()                                     |
|    +-> IF branch != selected_branch:                             |
|        +-> cmd = 'git checkout {branch}'                         |
|        +-> subprocess.run(cmd)                                   |
|        +-> IF returncode != 0:                                   |
|        |   +-> set_label("Cannot switch branch", tooltip)        |
|        +-> ELSE:                                                 |
|        |   +-> set_label("{repo} : {branch}", tooltip)           |
|        +-> update_branch(label=False)                            |
|                                                                  |
| 5. do_make_branch(branch)                                        |
|    |                                                             |
|    +-> get_branches()                                            |
|    +-> IF branch NOT in local_branches:                          |
|        +-> cmd = "git branch {branch}"                           |
|        +-> subprocess.run(cmd)                                   |
|        +-> IF returncode != 0:                                   |
|        |   +-> set_label("Cannot create branch", tooltip)        |
|        |   +-> RETURN Result::Err(error)                         |
|        +-> ELSE:                                                 |
|        |   +-> set_label("Branch created", tooltip)              |
|        +-> update_branch()                                       |
|        +-> RETURN Result::Ok()                                   |
+------------------------------------------------------------------+
```

### 2.3 Changes Tree Checkbox Toggle (`tree`)

```
+------------------------------------------------------------------+
| Widget: tree (QTreeWidget)                                       |
| Event: itemChanged                                               |
| Connection: self.tree.itemChanged.connect(self.tree_changed)     |
+------------------------------------------------------------------+
| Function Called: tree_changed(item, col)                         |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. tree_changed(item, col)                                       |
|    |                                                             |
|    +-> IF tree_list is empty:                                    |
|    |   +-> RETURN                                                |
|    |                                                             |
|    +-> IF item == tree_list[0]:  ("-- All --" item)              |
|        +-> IF item.checkState(0) == Qt.Checked:                  |
|        |   +-> select_all_changes()                              |
|        +-> ELSE:                                                 |
|            +-> deselect_all_changes()                            |
|                                                                  |
| 2. select_all_changes()                                          |
|    |                                                             |
|    +-> FOR each item in tree_list:                               |
|        +-> item.setCheckState(0, Qt.Checked)                     |
|                                                                  |
| 3. deselect_all_changes()                                        |
|    |                                                             |
|    +-> FOR each item in tree_list:                               |
|        +-> item.setCheckState(0, Qt.Unchecked)                   |
+------------------------------------------------------------------+
```

### 2.4 Changes Tree Context Menu (`tree`)

```
+------------------------------------------------------------------+
| Widget: tree (QTreeWidget)                                       |
| Event: customContextMenuRequested                                |
| Connection: self.tree.customContextMenuRequested.connect(        |
|             self.tree_context_menu)                              |
| Policy: Qt.CustomContextMenu                                     |
+------------------------------------------------------------------+
| Function Called: tree_context_menu(position)                     |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. tree_context_menu(position)                                   |
|    |                                                             |
|    +-> context_menu = QMenu(tree)                                |
|    +-> act1 = context_menu.addAction("Check diff")               |
|    +-> action = context_menu.exec_(tree.mapToGlobal(position))   |
|    +-> IF action == act1:                                        |
|        +-> filename = item.text(0)                               |
|        +-> IF file extension in DIFF_WHITELIST:                  |
|        |   +-> diff = do_git_diff(filename)                      |
|        |   +-> show_diff_viewer_dialog(filename, diff)           |
|        +-> ELSE:                                                 |
|            +-> [Binary file - diff not available]                |
+------------------------------------------------------------------+
| Diff Viewer Dialog:                                              |
| - Modal dialog with syntax-highlighted diff content              |
| - Only shown for whitelisted text file extensions                |
| - Whitelist defined at implementation time                       |
+------------------------------------------------------------------+
```

---

## 3. Keyboard Actions

### 3.1 Ctrl Key Press

```
+------------------------------------------------------------------+
| Widget: GitSardine (QMainWindow)                                     |
| Event: keyPressEvent                                             |
| Override: QMainWindow.keyPressEvent                              |
+------------------------------------------------------------------+
| Function Called: keyPressEvent(event)                            |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. keyPressEvent(event)                                          |
|    |                                                             |
|    +-> IF event.key() == Qt.Key_Control:                         |
|        +-> IF button_enabled:                                    |
|            +-> unlock_buttons()                                  |
|                                                                  |
| 2. unlock_buttons()                                              |
|    |                                                             |
|    +-> b_delete.setEnabled(True)                                 |
|    +-> b_clean.setEnabled(True)                                  |
|    +-> b_delete_file.setEnabled(True)                            |
+------------------------------------------------------------------+
```

### 3.2 Ctrl Key Release

```
+------------------------------------------------------------------+
| Widget: GitSardine (QMainWindow)                                     |
| Event: keyReleaseEvent                                           |
| Override: QMainWindow.keyReleaseEvent                            |
+------------------------------------------------------------------+
| Function Called: keyReleaseEvent(event)                          |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. keyReleaseEvent(event)                                        |
|    |                                                             |
|    +-> IF event.key() == Qt.Key_Control:                         |
|        +-> lock_buttons()                                        |
|                                                                  |
| 2. lock_buttons()                                                |
|    |                                                             |
|    +-> b_delete.setEnabled(False)                                |
|    +-> b_clean.setEnabled(False)                                 |
|    +-> b_delete_file.setEnabled(False)                           |
+------------------------------------------------------------------+
```

---

## 4. Timer Actions

### 4.1 Auto Status Update Timer (30 minutes)

```
+------------------------------------------------------------------+
| Timer: status_update_timer (Spin QThread)                        |
| Interval: 1800 seconds (30 minutes)                              |
| Signal: ended                                                    |
| Connection: status_update_timer.ended.connect(                   |
|             self.auto_update_status)                             |
+------------------------------------------------------------------+
| Function Called: auto_update_status()                            |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. auto_update_status()                                          |
|    |                                                             |
|    +-> updates_repo_status()  [refresh all repos]                |
|    +-> status_update_timer.start()  [restart timer]              |
+------------------------------------------------------------------+
```

### 4.2 Spinner Animation Timer

```
+------------------------------------------------------------------+
| Timer: spin (Spin QThread)                                       |
| Interval: 0.2 seconds                                            |
| Signal: ended                                                    |
| Connection: spin.ended.connect(self.update_spin)                 |
+------------------------------------------------------------------+
| Function Called: update_spin()                                   |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. update_spin()                                                 |
|    |                                                             |
|    +-> IF status_update.is_running:                              |
|    |   +-> spin_state += 1                                       |
|    |   +-> IF spin_state > 4:                                    |
|    |   |   +-> spin_state = 1                                    |
|    |   +-> path = "icon/spin/{spin_state}.png"                   |
|    |   +-> update_tree.setIcon(QIcon(path))                      |
|    |   +-> spin.start()  [continue animation]                    |
|    +-> ELSE:                                                     |
|        +-> update_tree.setIcon(QIcon("icon/spin/1.png"))         |
+------------------------------------------------------------------+
```

### 4.3 Progress Bar Animation

```
+------------------------------------------------------------------+
| Thread: progress (UpdateProgress QThread)                        |
| Signal: update_progress(int)                                     |
| Signal: ended                                                    |
| Connection: progress.update_progress.connect(self.update_progress)|
| Connection: progress.ended.connect(self.end_progress)            |
+------------------------------------------------------------------+
| Functions Called: update_progress(i), end_progress()             |
+------------------------------------------------------------------+
| Execution:                                                       |
|                                                                  |
| 1. UpdateProgress.run() [in thread]                              |
|    |                                                             |
|    +-> i = 0                                                     |
|    +-> WHILE bash.is_running:                                    |
|    |   +-> i += 5                                                |
|    |   +-> IF i > 100:                                           |
|    |   |   +-> i = 0                                             |
|    |   +-> sleep(0.1)                                            |
|    |   +-> update_progress.emit(i)                               |
|    +-> ended.emit()                                              |
|                                                                  |
| 2. update_progress(i) [on signal]                                |
|    |                                                             |
|    +-> ui.progress.setValue(i)                                   |
|                                                                  |
| 3. end_progress() [on ended signal]                              |
|    |                                                             |
|    +-> ui.progress.setValue(0)                                   |
|    +-> ui.progress.setVisible(False)                             |
|    +-> enable_buttons()                                          |
+------------------------------------------------------------------+
```

---

## 5. Signal/Slot Connections Summary

```
+------------------------------------------------------------------+
| Signal                          | Slot                           |
+---------------------------------+--------------------------------+
| update_tree.clicked             | updates_repo_status()          |
| b_extend.clicked                | on_b_extend()                  |
| b_delete.clicked                | on_delete_branch()             |
| b_update.clicked                | on_update()                    |
| b_pull.clicked                  | on_pull()                      |
| b_push.clicked                  | on_push()                      |
| b_delete_file.clicked           | on_delete_file()               |
| b_clean.clicked                 | do_restore()                   |
| b_reset.clicked                 | do_reset()                     |
| b_commit.clicked                | on_commit()                    |
| b_commit_push.clicked           | on_commit_push()               |
| b_ignore.clicked                | on_ignore()                    |
| b_merge.clicked                 | on_merge()                     |
| folder_tree.doubleClicked       | on_repo_selected()             |
| combo_branch.currentTextChanged | on_branch_choice()             |
| tree.itemChanged                | tree_changed()                 |
| tree.customContextMenuRequested | tree_context_menu()            |
| bash.strt                       | start_progress()               |
| bash.ret                        | bash_ret()                     |
| progress.update_progress        | update_progress()              |
| progress.ended                  | end_progress()                 |
| spin.ended                      | update_spin()                  |
| update_single_status            | single_status.start()          |
| status_update_timer.ended       | auto_update_status()           |
+---------------------------------+--------------------------------+
```

---

## 6. Button State Reference

### 6.1 Initially Disabled Buttons

```
+------------------------------------------------------------------+
| Button         | Initial State | Enable Condition               |
+----------------+---------------+--------------------------------+
| b_commit       | Disabled      | Repository selected            |
| b_commit_push  | Disabled      | Repository selected            |
| b_ignore       | Disabled      | Repository selected            |
| b_merge        | Disabled      | Repository selected            |
| b_pull         | Disabled      | Repository selected            |
| b_push         | Disabled      | Repository selected            |
| b_update       | Disabled      | Repository selected            |
| b_reset        | Disabled      | Repository selected            |
| b_delete       | Disabled      | Ctrl key held + repo selected  |
| b_clean        | Disabled      | Ctrl key held + repo selected  |
| b_delete_file  | Disabled      | Ctrl key held + repo selected  |
+----------------+---------------+--------------------------------+
```

### 6.2 Visibility Conditions

```
+------------------------------------------------------------------+
| Button         | Hidden When                                     |
+----------------+-------------------------------------------------+
| b_commit       | Branch is "master" or "main"                    |
| b_commit_push  | Branch is "master" or "main"                    |
| b_ignore       | Branch is "master" or "main"                    |
+----------------+-------------------------------------------------+
```
