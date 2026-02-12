# GitSardine Views Specification (Pixel-Perfect)

## 1. Main Window Properties

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Title             | "GitSardine"            |
| Icon              | gitsardine_icon.png     |
| Width (normal)    | 290 px              |
| Height            | 700 px              |
| Width (extended)  | 290 + extend px     |
| Resizable         | No (fixed size)     |
| Style             | Fusion              |
| Mouse Tracking    | Enabled             |
+-------------------+---------------------+
```

---

## 2. Fonts

```
+----------------------------------------------------------+
| Font specification: Use Qt defaults                      |
| No custom font family or size - inherit from system/Qt   |
+----------------------------------------------------------+
```

---

## 3. Color Palette (Dark Theme)

```
+---------------------------+-------------------+-------------+
| Element                   | QPalette Role     | RGB Value   |
+---------------------------+-------------------+-------------+
| Window Background         | Window            | #353535     |
| Window Text               | WindowText        | #FFFFFF     |
| Input Background          | Base              | #191919     |
| Alternate Background      | AlternateBase     | #353535     |
| Tooltip Background        | ToolTipBase       | #000000     |
| Tooltip Text              | ToolTipText       | #FFFFFF     |
| Text                      | Text              | #FFFFFF     |
| Button Background         | Button            | #353535     |
| Button Text               | ButtonText        | #FFFFFF     |
| Bright Text               | BrightText        | #FF0000     |
| Link                      | Link              | #2A82DA     |
| Selection Highlight       | Highlight         | #2A82DA     |
| Selection Text            | HighlightedText   | #000000     |
+---------------------------+-------------------+-------------+
```

---

## 3. Widget Specifications (Pixel-Perfect)

### 3.1 Update Repositories Button (`update_tree`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | update_tree         |
| X Position        | 10                  |
| Y Position        | 10                  |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | arrow-circle-315.png|
| Icon (animated)   | spin/1-4.png        |
| Tooltip           | "Update repositories|
|                   |  status"            |
+-------------------+---------------------+
```

### 3.2 Extend Button (`b_extend`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_extend            |
| X Position        | 257                 |
| Y Position        | 10                  |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon (collapsed)  | navigation-000-     |
|                   |  button-white.png   |
| Icon (extended)   | navigation-180-     |
|                   |  button-white.png   |
+-------------------+---------------------+
```

### 3.3 Folder Tree View (`folder_tree`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QTreeView           |
| Object Name       | folder_tree         |
| X Position        | 10                  |
| Y Position        | 40                  |
| Width             | 271                 |
| Height            | 231                 |
| Header Hidden     | True                |
| Model             | QStandardItemModel  |
| Selection Mode    | Single              |
| Interaction       | DoubleClick         |
+-------------------+---------------------+
| Extends by        | +extend pixels      |
+-------------------+---------------------+
```

### 3.4 Delete Branch Button (`b_delete`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_delete            |
| X Position        | 10                  |
| Y Position        | 280                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | cross-button.png    |
| Default Enabled   | False               |
| Unlock            | Ctrl key            |
| Tooltip           | "Delete current     |
|                   |  branch (Press ctrl |
|                   |  key for unlock)"   |
+-------------------+---------------------+
```

### 3.5 Update Changes Button (`b_update`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_update            |
| X Position        | 40                  |
| Y Position        | 280                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | arrow-circle-315.png|
| Tooltip           | "Update changes"    |
+-------------------+---------------------+
```

### 3.6 Branch Selector (`combo_branch`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QComboBox           |
| Object Name       | combo_branch        |
| X Position        | 70                  |
| Y Position        | 280                 |
| Width             | 150                 |
| Height            | 25                  |
| Items             | - Current branch    |
|                   | - Local branches    |
|                   | - <Remote branches> |
|                   | - "--new--"         |
+-------------------+---------------------+
```

### 3.7 Pull Button (`b_pull`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_pull              |
| X Position        | 225                 |
| Y Position        | 280                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | arrow-skip-270.png  |
| Tooltip           | "Git pull from      |
|                   |  origin"            |
+-------------------+---------------------+
```

### 3.8 Push Button (`b_push`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_push              |
| X Position        | 257                 |
| Y Position        | 280                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | arrow-skip-090.png  |
| Tooltip           | "Git push to origin"|
+-------------------+---------------------+
```

### 3.9 Repository Label (`label_repo`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QLabel              |
| Object Name       | label_repo          |
| X Position        | 10                  |
| Y Position        | 310                 |
| Width             | 271                 |
| Height            | 20                  |
| Text              | (dynamic)           |
| Alignment         | Qt::AlignCenter     |
| Format            | "{repo} : {branch}" |
+-------------------+---------------------+
```

### 3.10 Changes Tree Widget (`tree`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QTreeWidget         |
| Object Name       | tree                |
| X Position        | 10                  |
| Y Position        | 331                 |
| Width             | 271                 |
| Height            | 231                 |
| Context Menu      | Qt::NoContextMenu   |
|                   | (custom via code)   |
| Show Drop Ind.    | False               |
| Root Decorated    | False               |
| Header Visible    | False               |
| Columns           | 1                   |
+-------------------+---------------------+
| Extends by        | +extend pixels      |
+-------------------+---------------------+
```

### 3.11 Message Input (`msg`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QLineEdit           |
| Object Name       | msg                 |
| X Position        | 10                  |
| Y Position        | 570                 |
| Width             | 271                 |
| Height            | 25                  |
| Placeholder       | "Commit msg or new  |
|                   |  branch name"       |
+-------------------+---------------------+
```

### 3.12 Delete File Button (`b_delete_file`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_delete_file       |
| X Position        | 10                  |
| Y Position        | 600                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | cross.png           |
| Default Enabled   | False               |
| Unlock            | Ctrl key            |
| Tooltip           | "Delete selected    |
|                   |  file (Press ctrl   |
|                   |  key for unlock)"   |
+-------------------+---------------------+
```

### 3.13 Restore Button (`b_clean`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_clean             |
| X Position        | 40                  |
| Y Position        | 600                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | arrow-curve-180-    |
|                   |  left.png           |
| Default Enabled   | False               |
| Unlock            | Ctrl key            |
| Tooltip           | "Git restore .      |
|                   |  (restore files to  |
|                   |  last commit state  |
|                   |  Press ctrl key     |
|                   |  for unlock)"       |
+-------------------+---------------------+
```

### 3.14 Reset Button (`b_reset`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_reset             |
| X Position        | 70                  |
| Y Position        | 600                 |
| Width             | 25                  |
| Height            | 25                  |
| Text              | (empty)             |
| Icon              | disk--minus.png     |
| Tooltip           | "Git reset (cancel  |
|                   |  staged changes)"   |
+-------------------+---------------------+
```

### 3.15 Commit Button (`b_commit`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_commit            |
| X Position        | 100                 |
| Y Position        | 600                 |
| Width             | 61                  |
| Height            | 25                  |
| Text              | "Commit"            |
| Tooltip           | "Add & Commit file  |
|                   |  to branch"         |
| Hidden on         | master, main        |
+-------------------+---------------------+
```

### 3.16 Commit+Push Button (`b_commit_push`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_commit_push       |
| X Position        | 165                 |
| Y Position        | 600                 |
| Width             | 61                  |
| Height            | 25                  |
| Text              | "C + Push"          |
| Tooltip           | "Add & Commit file  |
|                   |  to branch + Push   |
|                   |  to origin"         |
| Hidden on         | master, main        |
+-------------------+---------------------+
```

### 3.17 Ignore Button (`b_ignore`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_ignore            |
| X Position        | 230                 |
| Y Position        | 600                 |
| Width             | 51                  |
| Height            | 25                  |
| Text              | "Ignore"            |
| Tooltip           | "Add this file to   |
|                   |  .gitignore file"   |
| Hidden on         | master, main        |
+-------------------+---------------------+
```

### 3.18 Merge Button (`b_merge`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QPushButton         |
| Object Name       | b_merge             |
| X Position        | 10                  |
| Y Position        | 630                 |
| Width             | 91                  |
| Height            | 25                  |
| Text              | "Merge"             |
| Tooltip           | "Merge selected     |
|                   |  branch into        |
|                   |  current branch"    |
+-------------------+---------------------+
```

### 3.19 From Label (`label_2`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QLabel              |
| Object Name       | label_2             |
| X Position        | 110                 |
| Y Position        | 631                 |
| Width             | 41                  |
| Height            | 20                  |
| Text              | "From"              |
+-------------------+---------------------+
```

### 3.20 Merge Source Selector (`combo_merge`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QComboBox           |
| Object Name       | combo_merge         |
| X Position        | 150                 |
| Y Position        | 630                 |
| Width             | 131                 |
| Height            | 25                  |
| Items             | All branches except |
|                   | current branch      |
+-------------------+---------------------+
```

### 3.21 Status Label (`label`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QLabel              |
| Object Name       | label               |
| X Position        | 10                  |
| Y Position        | 660                 |
| Width             | 271                 |
| Height            | 31                  |
| Mouse Tracking    | True                |
| Text              | (dynamic)           |
| Tooltip           | (dynamic - detailed |
|                   |  error info)        |
+-------------------+---------------------+
```

### 3.22 Progress Bar (`progress`)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Type              | QProgressBar        |
| Object Name       | progress            |
| X Position        | 10                  |
| Y Position        | 660                 |
| Width             | 271                 |
| Height            | 31                  |
| Value             | 0                   |
| Text Visible      | False               |
| Default Visible   | False               |
| Range             | 0-100               |
+-------------------+---------------------+
```

---

## 4. Complete Layout Grid (Pixel Coordinates)

```
Y=0    +------------------------------------------------------------------+
       |  X=10      X=257                                                 |
       |  +----+                                            +----+        |
       |  | U  |                                            | E  |        |
Y=10   |  +----+                                            +----+        |
       |  25x25                                             25x25         |
Y=35   +------------------------------------------------------------------+
Y=40   |  X=10                                              X=281         |
       |  +----------------------------------------------------------+    |
       |  |                                                          |    |
       |  |                    FOLDER TREE VIEW                      |    |
       |  |                      (271 x 231)                         |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  +----------------------------------------------------------+    |
Y=271  +------------------------------------------------------------------+
Y=280  |  X=10  X=40  X=70                X=220  X=225  X=257             |
       |  +--+ +--+ +----------------------+ +--+ +--+                    |
       |  |DL| |UP| |    BRANCH COMBO      | |PL| |PS|                    |
       |  +--+ +--+ +----------------------+ +--+ +--+                    |
       |  25   25         150x25             25   25                      |
Y=305  +------------------------------------------------------------------+
Y=310  |  X=10                                              X=281         |
       |  +----------------------------------------------------------+    |
       |  |              REPO LABEL (centered) 271x20                |    |
       |  +----------------------------------------------------------+    |
Y=330  +------------------------------------------------------------------+
Y=331  |  X=10                                              X=281         |
       |  +----------------------------------------------------------+    |
       |  |                                                          |    |
       |  |                   CHANGES TREE VIEW                      |    |
       |  |                      (271 x 231)                         |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  |                                                          |    |
       |  +----------------------------------------------------------+    |
Y=562  +------------------------------------------------------------------+
Y=570  |  X=10                                              X=281         |
       |  +----------------------------------------------------------+    |
       |  |                MESSAGE INPUT (271 x 25)                  |    |
       |  +----------------------------------------------------------+    |
Y=595  +------------------------------------------------------------------+
Y=600  |  X=10  X=40  X=70  X=100      X=165      X=230                   |
       |  +--+ +--+ +--+ +--------+ +--------+ +------+                   |
       |  |DF| |RS| |RT| | Commit | | C+Push | |Ignore|                   |
       |  +--+ +--+ +--+ +--------+ +--------+ +------+                   |
       |  25   25   25      61         61        51                       |
Y=625  +------------------------------------------------------------------+
Y=630  |  X=10         X=110  X=150                   X=281               |
       |  +--------+ From +---------------------------+                   |
       |  | Merge  |      |    MERGE SOURCE COMBO     |                   |
       |  +--------+      +---------------------------+                   |
       |     91      41              131x25                               |
Y=655  +------------------------------------------------------------------+
Y=660  |  X=10                                              X=281         |
       |  +----------------------------------------------------------+    |
       |  |          STATUS LABEL / PROGRESS BAR (271 x 31)          |    |
       |  +----------------------------------------------------------+    |
Y=691  +------------------------------------------------------------------+
       |                                                                  |
Y=700  +------------------------------------------------------------------+

Total: 290 x 700 pixels
```

---

## 5. Widget Summary Table

```
+------------------+------+------+-------+--------+
| Widget           | X    | Y    | Width | Height |
+------------------+------+------+-------+--------+
| update_tree      | 10   | 10   | 25    | 25     |
| b_extend         | 257  | 10   | 25    | 25     |
| folder_tree      | 10   | 40   | 271   | 231    |
| b_delete         | 10   | 280  | 25    | 25     |
| b_update         | 40   | 280  | 25    | 25     |
| combo_branch     | 70   | 280  | 150   | 25     |
| b_pull           | 225  | 280  | 25    | 25     |
| b_push           | 257  | 280  | 25    | 25     |
| label_repo       | 10   | 310  | 271   | 20     |
| tree             | 10   | 331  | 271   | 231    |
| msg              | 10   | 570  | 271   | 25     |
| b_delete_file    | 10   | 600  | 25    | 25     |
| b_clean          | 40   | 600  | 25    | 25     |
| b_reset          | 70   | 600  | 25    | 25     |
| b_commit         | 100  | 600  | 61    | 25     |
| b_commit_push    | 165  | 600  | 61    | 25     |
| b_ignore         | 230  | 600  | 51    | 25     |
| b_merge          | 10   | 630  | 91    | 25     |
| label_2          | 110  | 631  | 41    | 20     |
| combo_merge      | 150  | 630  | 131   | 25     |
| label            | 10   | 660  | 271   | 31     |
| progress         | 10   | 660  | 271   | 31     |
+------------------+------+------+-------+--------+
```

---

## 6. Extended Mode Modifications

```
+------------------------------------------+
| Component      | Normal   | Extended     |
+----------------+----------+--------------+
| Window Width   | 290      | 290 + extend |
| folder_tree W  | 271      | 271 + extend |
| tree Width     | 271      | 271 + extend |
| b_extend X     | 257      | 257 + extend |
+----------------+----------+--------------+

Default extend value: 190 pixels
Configurable in: config.json "extend" field
```

---

## 7. Icons Reference

### 7.1 Button Icons (Path: icon/)

```
+------------------+-------------------------------+
| Button           | Icon File                     |
+------------------+-------------------------------+
| update_tree      | arrow-circle-315.png          |
| update (spin 1)  | spin/1.png                    |
| update (spin 2)  | spin/2.png                    |
| update (spin 3)  | spin/3.png                    |
| update (spin 4)  | spin/4.png                    |
| b_extend (right) | navigation/navigation-000-    |
|                  |  button-white.png             |
| b_extend (left)  | navigation/navigation-180-    |
|                  |  button-white.png             |
| b_delete         | cross-button.png              |
| b_update         | arrow-circle-315.png          |
| b_pull           | arrow-skip-270.png            |
| b_push           | arrow-skip-090.png            |
| b_delete_file    | cross.png                     |
| b_clean          | arrow-curve-180-left.png      |
| b_reset          | disk--minus.png               |
+------------------+-------------------------------+
```

### 7.2 Status Icons (Path: icon/)

```
+------------------+-------------------------------+
| Status           | Icon File                     |
+------------------+-------------------------------+
| Checking         | arrow-circle-315.png          |
| Error            | exclamation-red.png           |
| Needs Pull       | drive-download.png            |
| Needs Push       | drive-upload.png              |
| Needs Commit     | disk--plus.png                |
| Clean            | document.png                  |
| Folder           | folder-horizontal.png         |
| Drive (root)     | drive.png                     |
+------------------+-------------------------------+
```

---

## 8. Modal Dialogs

### 8.1 User Not Defined

```
+------------------------------------------+
| Type              | QMessageBox         |
| Icon              | Information         |
| Title             | "User not define!"  |
| Text              | "user param not     |
|                   |  define in          |
|                   |  user.conf file!"   |
| Buttons           | OK                  |
| Trigger           | user == "user"      |
+-------------------+---------------------+
```

### 8.2 Space in Branch Name

```
+------------------------------------------+
| Type              | QMessageBox         |
| Icon              | Information         |
| Title             | "Spaces forbiben in |
|                   |  branch name!"      |
| Text              | "Spaces are forbiden|
|                   |  in space name! use |
|                   |  '_' instead!       |
|                   |  Do you want to     |
|                   |  commit instead?"   |
| Buttons           | OK                  |
+-------------------+---------------------+
```

### 8.3 Enter Commit Message

```
+------------------------------------------+
| Type              | QMessageBox         |
| Icon              | Information         |
| Title             | "Enter commit msg!!"|
| Text              | "Commit message is  |
|                   |  empty, please      |
|                   |  define a commit    |
|                   |  message!"          |
| Buttons           | OK                  |
+-------------------+---------------------+
```

### 8.4 Enter Branch Name

```
+------------------------------------------+
| Type              | QMessageBox         |
| Icon              | Information         |
| Title             | "Enter branch name!"|
| Text              | "Branch name is     |
|                   |  empty, please      |
|                   |  define a commit    |
|                   |  message!"          |
| Buttons           | OK                  |
+-------------------+---------------------+
```

### 8.5 Username in Branch

```
+------------------------------------------+
| Type              | QMessageBox         |
| Icon              | Information         |
| Title             | "Branch name!"      |
| Text              | "New branch name    |
|                   |  might contain your |
|                   |  username!          |
|                   |  ex: {user}_dev or  |
|                   |  dev_{user}"        |
| Buttons           | OK                  |
+-------------------+---------------------+
```

### 8.6 Diff Viewer Dialog

**Note: New feature for C++ (not implemented in Python - Python only shows diff in tooltip)**

```
+------------------------------------------+
| Type              | QDialog             |
| Title             | "Diff: {filename}"  |
| Modal             | Yes                 |
| Resizable         | Yes                 |
| Min Width         | 600 px              |
| Min Height        | 400 px              |
+-------------------+---------------------+
| Content           | QTextEdit (readonly)|
| Font              | Monospace           |
| Syntax            | Diff highlighting   |
|                   | (+green, -red)      |
+-------------------+---------------------+
| Buttons           | Close               |
+-------------------+---------------------+
| Availability      | Only for whitelisted|
|                   | text file extensions|
|                   | (DATA_MODELS.md 2.3)|
+-------------------+---------------------+
```

---

## 9. Tree Item Specifications

### 9.1 Folder Tree Item (Folder class)

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Base Class        | QStandardItem       |
| Editable          | False               |
| Display           | Folder/repo name    |
| Icon              | Based on status     |
+-------------------+---------------------+

Attributes:
+-------------------+---------------------+
| os_path           | Full filesystem path|
| path              | Relative path       |
| name              | Display name        |
| depth             | Nesting level       |
| is_repo           | Boolean             |
| need_pull         | Boolean             |
| need_push         | Boolean             |
| need_commit       | Boolean             |
| status_error      | Boolean             |
| status_checked    | Boolean             |
+-------------------+---------------------+
```

### 9.2 Changes Tree Item

```
+------------------------------------------+
| Property          | Value               |
+-------------------+---------------------+
| Base Class        | QTreeWidgetItem     |
| Flags             | ItemIsUserCheckable |
| Column 0 Text     | Filename            |
| Column 0 Tooltip  | "filename\n<diff>"  |
| Check State       | Unchecked (default) |
+-------------------+---------------------+
```

### 9.3 Changes Tree Header

```
+------------------------------------------+
| Type              | QTreeWidgetItem     |
| Text              | "---- Modified      |
|                   |  files ----"        |
| Checkable         | No                  |
+-------------------+---------------------+
```

### 9.4 Select All Item

```
+------------------------------------------+
| Type              | QTreeWidgetItem     |
| Text              | "-- All --"         |
| Checkable         | Yes                 |
| Position          | First in tree_list  |
| Action            | Toggle all checks   |
+-------------------+---------------------+
```

### 9.5 Cached Header

```
+------------------------------------------+
| Type              | QTreeWidgetItem     |
| Text              | "---- Cached        |
|                   |  files ----"        |
| Checkable         | No                  |
+-------------------+---------------------+
```
