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

### [ ] Fix TYPE_CHECKING block handling - indentation errors with inlined imports

**Problem**: Python-inliner incorrectly inlines code inside `if TYPE_CHECKING:` blocks, causing indentation errors in the output file.

**Technical Details**:
- `TYPE_CHECKING` is a special typing construct in Python (from `typing.TYPE_CHECKING`)
- Code inside `if TYPE_CHECKING:` blocks should only contain type hints and is removed at runtime
- Imports inside TYPE_CHECKING blocks should NOT be inlined as executable code
- When these imports are inlined, they appear with incorrect indentation, causing `IndentationError`

**Observed Failure** (from Pyra project build):
```
File: pyra-inlined.py, line 1525
    LITELLM_API_KEY,
IndentationError: unexpected indent
```

**Root Cause Analysis**:
1. Module `llm_api/provider_config.py` contains:
   ```python
   from typing import TYPE_CHECKING

   if TYPE_CHECKING:
       from llm_api.environment import (
           LITELLM_API_KEY,
           OPENAI_API_KEY,
           DEEPSEEK_API_KEY,
           ZAI_API_KEY,
           XAI_API_KEY,
           GEMINI_API_KEY,
       )
   ```

2. Python-inliner inlines both the TYPE_CHECKING import block AND the actual module `llm_api/environment.py`

3. The inlined code appears as:
   ```python
   # ↑↑↑ inlined submodule: llm_api.environment
       LITELLM_API_KEY,       # ← WRONG INDENTATION
       OPENAI_API_KEY,
       DEEPSEEK_API_KEY,
       ZAI_API_KEY,
       XAI_API_KEY,
       GEMINI_API_KEY,
   )  # ← This parenthesis has no matching open
   ```

4. This creates a syntax error because:
   - The imports have incorrect indentation (should not be indented inside TYPE_CHECKING block)
   - The closing `)` has no matching opening `(` in the inlined code
   - The code structure is semantically invalid for Python runtime

**Expected Behavior**:
- `if TYPE_CHECKING:` blocks should be **skipped entirely** during inlining
- Only actual executable code (outside TYPE_CHECKING) should be inlined
- Type hints inside TYPE_CHECKING blocks are for static analysis tools (mypy, etc.) only
- At runtime, `TYPE_CHECKING` is `False`, so code inside these blocks is never executed

**Implementation Requirements**:

**Parser Enhancement**:
- [ ] Detect `if TYPE_CHECKING:` and `if TYPE_CHECKING:` blocks during import scanning
- [ ] Mark all code between `if TYPE_CHECKING:` and its dedent as "type-check-only" code
- [ ] Skip inlining all imports and code inside TYPE_CHECKING blocks
- [ ] Preserve TYPE_CHECKING blocks in their original form (don't inline their contents)

**Edge Cases to Handle**:
- [ ] Nested TYPE_CHECKING blocks (though uncommon)
- [ ] TYPE_CHECKING with complex conditions (e.g., `if TYPE_CHECKING or SOME_DEBUG_FLAG:`)
- [ ] `elif TYPE_CHECKING:` and `else:` blocks related to TYPE_CHECKING
- [ ] Imports that appear both inside and outside TYPE_CHECKING blocks
- [ ] Type aliases defined inside TYPE_CHECKING blocks

**Validation Tests**:
- [ ] Add test case: Module with TYPE_CHECKING imports that shouldn't be inlined
- [ ] Add test case: Module with mixed TYPE_CHECKING and normal imports
- [ ] Verify output doesn't contain inlined TYPE_CHECKING block contents
- [ ] Verify output Python file runs without IndentationError
- [ ] Verify type hints (if any) remain in valid form for static analyzers

**Example Test Scenario**:
```python
# test_module_a.py
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from other_module import SomeClass  # Should NOT be inlined

def function_a():
    return 42
```

```python
# test_module_b.py
from test_module_a import function_a

def main():
    result = function_a()
    print(result)
```

**Expected Inlined Output**:
```python
# test_module_a.py
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from other_module import SomeClass  # ← Should remain as-is, NOT inlined

def function_a():
    return 42
# ↑↑↑ inlined submodule: test_module_a

def main():
    result = function_a()
    print(result)
```

**Incorrect Current Output** (what's happening now):
```python
# test_module_a.py
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from other_module import SomeClass

# ↑↑↑ inlined submodule: test_module_a
    SomeClass  # ← WRONG! IndentationError, shouldn't be here

def function_a():
    return 42

def main():
    result = function_a()
    print(result)
```

**Impact**:
- **Critical**: Makes inlined code unusable when source code uses TYPE_CHECKING (very common pattern)
- **Blocker**: Prevents Python Inliner from working with modern Python projects that use type hints
- **Severity**: High - affects any project with type checking infrastructure

**Reference Context**:
- Pyra project test run: `/Users/billdoughty/src/wdd/python/agents`
- Command used: `python-inliner pyra.py pyra-inlined.py agent,llm_api,modules,pyra,tools -v`
- Output file: `pyra-inlined.py` (533KB, 50+ modules inlined)
- Failing line: 1525 (IndentationError on dangling import names)

**Research Notes**:
- Python PEP 484 introduced TYPE_CHECKING for type hints without runtime import cost
- Common pattern: "import only for type annotations" to avoid circular imports
- Many modern Python projects use this pattern extensively
- The inliner should respect Python's semantics around TYPE_CHECKING

---

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
