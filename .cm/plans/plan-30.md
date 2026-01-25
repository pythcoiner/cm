Add a Step 0 prerequisites check to the /cm wizard in `assets/cm.md`:

1. Find the section after "Important: Interactive Flow" (around line 20)

2. Insert new Step 0 before Step 1:

```markdown
---

## Step 0: Prerequisites Check

Before starting the wizard, verify that `cm init` has been run:

1. Check if `.claude/commands/cm.md` exists (you're reading this, so it does)
2. Check if `.cm/agents/` directory exists with template files

If `.cm/agents/` is missing or incomplete, inform the user:

> **Prerequisites not met.**
>
> Please run `cm init` in your terminal first to set up the required files:
>
> ```bash
> cm init
> ```
>
> This will create:
> - `.claude/commands/` - Claude Code command files
> - `.cm/agents/` - Agent template files
>
> After running `cm init`, return here and run `/cm` again.

Wait for user confirmation that they've run `cm init` before proceeding.

If prerequisites are met, proceed to Step 1.

---
```

3. Run `cargo build` to re-embed the updated asset
4. Run `cm init --force` to update installed command files
5. Verify the change is correctly embedded
