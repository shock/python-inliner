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

### [ ] Handle renamed imports (as keyword) - incorrect inlining causes NameError

**Problem**: Import statements with aliases (`from X import Y as Z`) are not handled correctly, causing NameError at runtime.

**Technical Details**:
- Renamed imports use `as` keyword: `from mylib import helper as h`
- This creates an alias in the local scope
- Current regex captures the alias but ignores it
- When inlined, the alias is not created, causing undefined name errors

**Observed Failure**:
```python
# Source code:
from mylib import helper as h

def foo():
    return h()  # Uses alias 'h'

# Current inliner output:
def foo():
    # ↓↓↓ inlined submodule: mylib
    def helper():
        return 'hello'
    # ↑↑↑ inlined submodule: mylib
    return h()  # ← ERROR! NameError: name 'h' is not defined
```

**Root Cause**:
1. Regex at line 229 captures: `r"(?m)^([ \t]*)from\s+((?:{})\S*)\s+import\s+(.+)$"`
   - Group 1: indentation
   - Group 2: module name
   - Group 3: everything after 'import' (includes alias)
2. Line 270: `#[allow(unused)] let imports = &cap[3];`
   - Captured alias text is marked as unused and ignored
   - Inliner always inlines entire module
   - Alias is never created in output

**Expected Behavior**:
```python
# Expected inliner output:
def foo():
    # ↓↓↓ inlined submodule: mylib
    def helper():
        return 'hello'
    h = helper  # ← Create the alias!
    # ↑↑↑ inlined submodule: mylib
    return h()  # ✓ Works correctly
```

**Complexity Notes**:
- Multi-imports: `from X import a, b, c as d, e`
- Renaming multiple: `from X import a as x, b as y, c as z`
- Parenthesized: `from X import (a, b, c)`
- Wildcard imports: `from X import *`
- Current TODO comment acknowledges complexity: "TODO: handle specific imports? non-trivial"

**Implementation Requirements**:
- [ ] Parse import statements to extract original names and aliases
- [ ] Handle multiple imports in one statement
- [ ] Handle parenthesized multi-line imports
- [ ] Create alias assignments after inlining (when applicable)
- [ ] Preserve original Python semantics

**Edge Cases to Handle**:
- [ ] `from X import *` (no aliases, but still needs consideration)
- [ ] `from X import (a, b, c)` (parenthesized)
- [ ] `from X import a, b as c, d` (mixed)
- [ ] Multi-line: `from X import (a, b, c,)`
- [ ] Empty parenthesized: `from X import ()` (should remove?)

**Research Tasks**:
- [ ] Study how Python's import system handles aliases
- [ ] Check if there are existing Python preprocessors that handle this
- [ ] Research `ast` module for robust parsing vs regex
- [ ] Evaluate if we should only inline imported names vs entire module

**Impact**:
- **High**: Causes immediate runtime errors for any code using renamed imports
- **Common**: `as` keyword is widely used in Python code
- **Blocker**: Prevents inliner from working with codebases using import aliases

---

### [ ] Fix function-level import handling - incorrect indentation after module inlining

**Problem**: Function-level imports inside methods are being inlined at wrong indentation levels, causing IndentationError.

**Technical Details**:
- Function-level imports are imports inside function bodies (not at module top)
- Common pattern: `from module import Something` inside a function
- Used to avoid circular imports or for lazy loading
- When inlined, these imports appear with incorrect indentation relative to surrounding code

**Observed Failure** (from Pyra project build):
```
File: pyra-inlined.py, line 3433
    payload = {
IndentationError: unexpected indent
```

**Root Cause Analysis**:
1. Module `agent/callbacks.py` contains `ToolCallbacks` class with methods:
   ```python
   async def call_llm_light(self, prompt: str, temperature: float = 0.0):
       """Call LLM using light provider config (fast, less performant)."""
       from agent.llm_response import LLMResponse  # ← Function-level import
       from llm_api import get_chat_completions

       payload = {
           "model": self._llm_provider_config_light.model,
           ...
       }
   ```

2. Python-inliner processes these function-level imports and inlines the modules

3. The inlined code appears incorrectly after `__all__` statements:
   ```python
   ]
       # ↑↑↑ inlined package: llm_api

       payload = {  # ← WRONG INDENTATION
           "model": self._llm_provider_config_light.model,
           ...
       }
   ```

4. This creates IndentationError because:
   - The function code is not at the correct indentation level
   - It appears as loose code after the `__all__` statement
   - The indentation context is lost during inlining

**Expected Behavior**:
- Function-level imports should be inlined INSIDE the function body
- Indentation should be preserved from original source
- The function structure should remain intact
- Code should only appear within the function's indentation scope

**Implementation Requirements**:

**Parser Enhancement**:
- [ ] Detect function-level imports (imports inside function/method bodies)
- [ ] Track indentation level when entering/leaving functions
- [ ] When inlining function-level imports, maintain function's indentation context
- [ ] Ensure inlined code appears at correct indentation level within function

**Edge Cases to Handle**:
- [ ] Nested functions (imports inside nested functions)
- [ ] Multiple function-level imports in same function
- [ ] Imports at different indentation levels (class methods vs. nested functions)
- [ ] Function-level imports in async functions, generators, etc.
- [ ] Mixed imports (some at module level, some at function level)

**Validation Tests**:
- [ ] Add test case: Module with function-level imports
- [ ] Add test case: Class methods with function-level imports
- [ ] Verify output maintains correct indentation
- [ ] Verify output Python file runs without IndentationError
- [ ] Verify function behavior is preserved after inlining

**Example Test Scenario**:
```python
# test_module.py
def my_function():
    """Test function with function-level imports."""
    from other_module import SomeClass  # ← Should be inlined inside function

    obj = SomeClass()
    return obj.value
```

```python
# main.py
from test_module import my_function

def main():
    result = my_function()
    print(result)
```

**Expected Inlined Output**:
```python
# test_module.py
def my_function():
    """Test function with function-level imports."""
    # ↓↓↓ inlined submodule: other_module

    class SomeClass:  # ← Should be at function indentation level
        def __init__(self):
            self.value = 42
    # ↑↑↑ inlined submodule: other_module

    obj = SomeClass()
    return obj.value
# ↑↑↑ inlined submodule: test_module

def main():
    result = my_function()
    print(result)
```

**Impact**:
- **Critical**: Makes inlined code unusable when source uses function-level imports (common pattern)
- **Blocker**: Prevents Python Inliner from working with codebases using lazy/circular import avoidance
- **Severity**: High - affects many Python projects with complex import patterns

**Reference Context** (Real Project Files):
- Pyra project location: `/Users/billdoughty/src/wdd/python/agents`
- Source file demonstrating issue: `/Users/billdoughty/src/wdd/python/agents/agent/callbacks.py`
  - Contains `ToolCallbacks` class with `call_llm_light()` method (line 163)
  - Function-level imports at lines 165-166:
    ```python
    from agent.llm_response import LLMResponse
    from llm_api import get_chat_completions
    ```
- Entry point file: `/Users/billdoughty/src/wdd/python/agents/pyra.py`
- Inlined modules: `agent,llm_api,modules,pyra,tools`
- Command used: `python-inliner pyra.py pyra-inlined.py agent,llm_api,modules,pyra,tools -v`
- Output file: `/Users/billdoughty/src/wdd/python/agents/pyra-inlined.py` (533KB)
- Build output shows: "Stripping TYPE_CHECKING block:" working correctly
- Failing line: 3433 (IndentationError: unexpected indent on `payload = {`)
- Other modules with TYPE_CHECKING that were handled correctly:
  - `/Users/billdoughty/src/wdd/python/agents/agent/callbacks.py` (line 18) - Stripped
  - `/Users/billdoughty/src/wdd/python/agents/agent/ui_manager.py` - Stripped
  - `/Users/billdoughty/src/wdd/python/agents/agent/token_manager.py` - Stripped
  - `/Users/billdoughty/src/wdd/python/agents/pyra/slash_command_handler.py` - Stripped

**Test Files for Validation**:
- Can test with real project: `python-inliner pyra.py test-output.py agent,llm_api,modules,pyra,tools -v`
- Can examine specific module: `python-inliner agent/callbacks.py test-output.py agent,llm_api -v`
- Can create minimal reproducible test case in `/Users/billdoughty/src/wdd/rust/python-inliner/test/`

**Research Notes**:
- Function-level imports are valid Python (PEP 8 compliant)
- Common pattern for avoiding circular dependencies
- Used for lazy loading (imports only when needed)
- Inliner must preserve function context (indentation) during inlining
- This pattern appears in multiple files across `/Users/billdoughty/src/wdd/python/agents/` project

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

**Option 2 (Alternative)**: If using Bash, be aware that the sed command syntax varies by platform. On macOS, use:
```bash
sed -i '' '/^$/{N;/^\n$/D;}' TODO.md
```

**IMPORTANT**: Always verify the file structure after cleanup by reading the file again.
