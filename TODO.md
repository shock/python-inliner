# IMPROVEMENTS for Python Inliner

This is a TODO list for Python Inliner improvements and features that I'm working on or planning.  Python Inliner is allowed to edit this document.

The H2 "TODO" section below is the main list of things to do.  Each todo item is a '###' H3 section with a checkbox as shown in the example below.

<example_item>
### [ ] Example TODO item

Text here is general explanation about the TODO item, and any notes.
Bullet points below are subtasks if the TODO item requires multiple steps.

- [ ] Demonstrates a subtask that isn't complete
- [x] Demonstrates a subtask that is complete
</example_item>

## TODO

## **CRITICAL INSTRUCTIONS FOR AGENTS** - READ CAREFULLY

**YOU MUST READ AND UNDERSTAND THESE INSTRUCTIONS BEFORE MAKING ANY CHANGES TO THIS FILE.**

### GENERAL RULES
- You can add items, update descriptions/notes, and mark items/subtasks as complete
- DO NOT remove items unless explicitly requested!

### TO-DO MANAGEMENT SYSTEM

**We now use two separate documents:**
1. **TODO.md** - Contains only incomplete items (items with `[ ]` checkboxes)
2. **DONE.md** - Contains only completed items (items with `[x]` checkboxes)

### COMPLETING AN ITEM (changing [ ] → [x]):
1. **MOVE THE ENTIRE ITEM** from TODO.md to DONE.md underneath the `## COMPLETED` section
   - **IMPORTANT**: When reading DONE.md file, use a limit of 100 lines. You don't need to be concerned with the whole document. It is rather large and we don't want you to load it into your context unnecessarily.
2. **ALWAYS add the item as the FIRST item** UNDER the `## COMPLETED` section in DONE.md
3. **TIMESTAMP the item** when moving it to DONE.md by adding a completion timestamp at the end of the item description
   - Format: `**Completed:** YYYY-MM-DD HH:MM:SS`
   - Example: `**Completed:** 2025-12-15 14:30:45`
   - Use 24-hour format with leading zeros
   - Timezone is assumed to be local system time
4. The checkbox should already be changed from `[ ]` to `[x]` when moving
5. Sub-tasks should be marked as complete when moved along with the parent item

### REOPENING AN ITEM (changing [x] → [ ]):
1. **MOVE THE ENTIRE ITEM** from DONE.md back to TODO.md
2. **ALWAYS add the item as the FIRST item** in the `## TODO` section in TODO.md
3. Remove the completion timestamp from the item
4. The checkbox should be changed from `[x]` to `[ ]` when moving
5. Sub-tasks should be marked as incomplete when moved along with the parent item

### ADDING NEW ITEMS:
- Add new items to the **TOP** of the "## TODO" section in TODO.md, so they are always the first item
- The TODO section starts at the line `## TODO` - NOT in the preface area above it

### SUB-TASKS:
- Sub-tasks can be marked as complete with `[x]` without making the parent item complete
- Completed sub-tasks should be moved below the last incomplete sub-task
- New sub-tasks should be added to the **TOP** of their parent item's sub-task list

### FILE STRUCTURE:
- **INCOMPLETE ITEMS** are in the **## TODO** section of **TODO.md**
- **COMPLETED ITEMS** are in the **## COMPLETED** section of **DONE.md**
- Items should be moved between files when their status changes

**REMEMBER: CHANGING CHECKBOX STATUS WITHOUT MOVING THE ITEM TO THE CORRECT FILE IS WRONG!**

**CRITICAL**: FINAL STEP - After making any changes, always clean up extra blank lines:

**Option 1 (Recommended)**: Use the Edit tool to manually remove extra blank lines by reading the file and using Edit/MultiEdit to fix specific blank line issues.

**Option 2 (Alternative)**: If using Bash, be aware that sed command syntax varies by platform. On macOS, use:
```bash
sed -i '' '/^$/{N;/^\n$/D;}' TODO.md
```

**IMPORTANT**: Always verify the file structure after cleanup by reading the file again.
