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

### [ ] Fix module-level code indentation preservation during inlining

**Problem**: Python-inliner incorrectly preserves indentation from import context instead of maintaining original module-level indentation (column 0), causing IndentationError when module-level code is inlined into indented contexts.

**Technical Details**:
- When module-level code (indentation level 0) is imported into a function/class/other indented context, python-inliner **inherits the indentation from the import location**
- This causes module-level code (constants, variables, function definitions) to be incorrectly indented
- Module-level code should **always maintain its original indentation level 0**, regardless of where it's imported
- The inliner currently adds indentation based on the insertion point rather than preserving the inlined code's original indentation

**Observed Failure** (from Pyra project build):
```
File: pyra-inlined.py, line 1670
    LITELLM_API_KEY,
IndentationError: unexpected indent
```

**Real-World Test Case**:
- **Project**: Pyra AI agent at `/Users/billdoughty/src/wdd/python/agents`
- **Build command**: `python-inliner pyra.py pyra-inlined.py agent,llm_api,modules,pyra,tools -v`
- **Output file**: `pyra-inlined.py` (533KB, 50+ modules inlined)

**Specific Failing Import Chain**:
1. `agent/callbacks.py` contains:
   ```python
   async def call_llm_light(self, prompt: str, temperature: float = 0.0):
       """Call LLM using light provider config."""
       # This function is indented 4 spaces inside class
   ```

2. Within this function, `llm_api/provider_config.py` is imported:
   ```python
   # Inside call_llm_light(), at indentation level 4:
   from llm_api import provider_config  # ← Import is indented 4 spaces
   ```

3. `llm_api/provider_config.py` imports from `llm_api/environment.py`:
   ```python
   # This is MODULE-LEVEL code (indentation 0):
   from llm_api.environment import (
       LITELLM_API_KEY,
       OPENAI_API_KEY,
       DEEPSEEK_API_KEY,
       ZAI_API_KEY,
       XAI_API_KEY,
       GEMINI_API_KEY,
   )
   ```

4. `llm_api/environment.py` contains module-level constants:
   ```python
   # This is MODULE-LEVEL code (indentation 0):
   import os

   LITELLM_API_KEY = os.getenv("LITELLM_API_KEY") or "sk-AJ2txPPokrC4QmPDt3thOg"
   OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")
   DEEPSEEK_API_KEY = os.getenv("DEEPSEEK_API_KEY")
   ZAI_API_KEY = os.getenv("ZAI_API_KEY")
   XAI_API_KEY = os.getenv("XAI_API_KEY")
   GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")
   ```

5. **Current Incorrect Inlined Output**:
   ```python
   async def call_llm_light(self, prompt: str, temperature: float = 0.0):
       """Call LLM using light provider config."""
       # ↓↓↓ inlined submodule: llm_api.environment
       import os

       LITELLM_API_KEY = os.getenv("LITELLM_API_KEY")  # ← WRONG! 8 spaces
       OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")      # ← WRONG! Should be 0 spaces
       DEEPSEEK_API_KEY = os.getenv("DEEPSEEK_API_KEY")
       ZAI_API_KEY = os.getenv("ZAI_API_KEY")
       XAI_API_KEY = os.getenv("XAI_API_KEY")
       GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")

       # ↑↑↑ inlined submodule: llm_api.environment
           LITELLM_API_KEY,  # ← WRONG! 12 spaces (inheriting from nested context)
           OPENAI_API_KEY,      # ← This is dangling from import statement in provider_config
           DEEPSEEK_API_KEY,
           ZAI_API_KEY,
           XAI_API_KEY,
           GEMINI_API_KEY,
       )
   ```

6. **Expected Correct Inlined Output**:
   ```python
   async def call_llm_light(self, prompt: str, temperature: float = 0.0):
       """Call LLM using light provider config."""
       # ↓↓↓ inlined submodule: llm_api.environment
       import os

   LITELLM_API_KEY = os.getenv("LITELLM_API_KEY")  # ← CORRECT! 0 spaces
   OPENAI_API_KEY = os.getenv("OPENAI_API_KEY")      # ← CORRECT!
   DEEPSEEK_API_KEY = os.getenv("DEEPSEEK_API_KEY")
   ZAI_API_KEY = os.getenv("ZAI_API_KEY")
   XAI_API_KEY = os.getenv("XAI_API_KEY")
   GEMINI_API_KEY = os.getenv("GEMINI_API_KEY")
   # ↑↑↑ inlined submodule: llm_api.environment
   ```

**Root Cause Analysis**:
1. Python-inliner tracks indentation level when scanning source file
2. When inlining a module, it uses the indentation level of the **import statement** as the base indentation for all inlined code
3. This is fundamentally incorrect - module-level code has its own indentation context
4. The inliner should:
   - Parse the inlined file independently to determine its indentation structure
   - Preserve the original indentation of each line exactly as written
   - Only adjust for the inlining marker comments (`↓↓↓ inlined submodule:` and `↑↑↑ inlined submodule:`)
   - NOT add indentation based on where the import statement appeared

**Indentation Semantics in Python**:
- **Module-level code** (indentation 0): Imports, variable assignments, function definitions, class definitions at top level
- **Class-level code** (indentation 4): Methods, class variables
- **Function-level code** (indentation 4): Local variables, return statements
- **Nested code** (indentation 8+): If statements, loops, try/except blocks

When inlining module-level code into any context (module, class, function), it **must remain at indentation 0**.

**Impact**:
- **Critical**: Makes inlined code unusable when source code has module-level imports (very common)
- **Blocker**: Prevents Python Inliner from working with virtually all Python projects
- **Severity**: High - affects any project with multiple modules and module-level imports
- **Scope**: Not just TYPE_CHECKING issue - affects ALL module-level code inlining

**Implementation Requirements**:

**Parser Enhancement**:
- [ ] When inlining a module, parse it independently to extract original indentation of each line
- [ ] Preserve the exact indentation of each line as it appears in the source file
- [ ] Do NOT apply indentation from the import context to inlined code
- [ ] Track indentation separately for:
  - The import statement context (where import appears)
  - The inlined code context (original indentation in source file)

**Indentation Tracking Strategy**:
- [ ] Use regex to capture leading whitespace of each line in source file: `^(\s*)`
- [ ] Store this original indentation with each line during parsing
- [ ] When writing inlined content, restore original indentation + inlining comment indentation
- [ ] Inlining comments (`↓↓↓ inlined submodule:`) should be at import context indentation
- [ ] Inlined code should maintain its original indentation exactly

**Example Implementation Logic**:
```rust
// Pseudo-code for indentation preservation
for line in source_file.lines {
    original_indent = line.leading_whitespace;  // Capture original indentation (e.g., 0, 4, 8 spaces)

    if line.is_import_statement() {
        import_context_indent = original_indent;  // Track where import appeared
    } else if line.is_being_inlined() {
        // When inlining this line:
        // Use ORIGINAL indent from source file, NOT import context
        output_line = " ".repeat(import_context_indent) +  // For inlining comment
                      "# ↓↓↓ inlined submodule: " + module_name + "\n" +
                      line.with_original_indentation(original_indent);  // Preserve original!
    }
}
```

**Edge Cases to Handle**:
- [ ] Module imported multiple times (should preserve indentation each time)
- [ ] Import inside class/method but module being imported has mixed indentation levels
- [ ] Multi-line imports with parentheses (very common pattern)
- [ ] Imports with trailing comments or inline comments
- [ ] Code inside conditionals or try/except blocks at module level
- [ ] Decorator functions at module level
- [ ] If statements and other block structures at module level

**Validation Tests**:

**Test Case 1: Module-level constants**:
```python
# test_module_a.py
CONSTANT_A = "value"  # Indentation 0
CONSTANT_B = "value"  # Indentation 0

def func_a():  # Indentation 0
    return CONSTANT_A
```

```python
# test_module_b.py
def function_b():
    # Indentation 4
    from test_module_a import func_a  # Import at indentation 4

    return func_a()
```

**Expected Inlined Output**:
```python
def function_b():
    # Indentation 4
    # ↓↓↓ inlined submodule: test_module_a  # Comment at indent 4
    # ↓↓↓ inlined submodule: test_module_a
CONSTANT_A = "value"  # ← Indentation 0 (preserved!)
CONSTANT_B = "value"  # ← Indentation 0 (preserved!)

def func_a():  # ← Indentation 0 (preserved!)
    return CONSTANT_A
# ↑↑↑ inlined submodule: test_module_a

    return func_a()
```

**Test Case 2: Multi-level imports (Pyra scenario)**:
Use Pyra project as test case:
```bash
cd /Users/billdoughty/src/wdd/python/agents
python-inliner pyra.py test_output.py agent,llm_api,modules -v
python test_output.py  # Should run without IndentationError
```

**Test Case 3: Nested import contexts**:
```python
# outer.py
from inner import something

class OuterClass:  # Indentation 4
    def method(self):  # Indentation 8
        from middle import OtherThing  # Import at indentation 12

        return OtherThing()
```

```python
# middle.py
from inner import something_else  # Indentation 0

class MiddleClass:  # Indentation 4
    pass
```

```python
# inner.py
SOME_CONST = 123  # Indentation 0 - should preserve when imported into any context
```

**Expected**: `SOME_CONST` always has indentation 0, regardless of import context (4, 8, 12 spaces).

**Test Case 4: Module with mixed indentation**:
```python
# mixed_indent.py
LEVEL_0 = "constant"  # Indentation 0

def level_0_function():  # Indentation 0
    """Function at module level."""
    return "value"

class Level_0_Class:  # Indentation 0
    def method(self):  # Indentation 4
        return LEVEL_0
```

When imported anywhere, all these should maintain their original indentation levels.

**Integration with Existing Features**:
- [ ] Ensure TYPE_CHECKING stripping still works correctly
- [ ] Verify circular import detection still works
- [ ] Test with verbose mode output (`-v` flag)
- [ ] Verify release mode (`--release` flag) indentation handling
- [ ] Check that import consolidation in release mode doesn't break indentation

**Testing Strategy**:
1. **Unit Tests**: Add test cases for indentation preservation:
   - Test module-level imports preserve indentation 0
   - Test nested imports preserve correct indentation at each level
   - Test multi-line imports preserve indentation correctly
   - Test mixed indentation modules are inlined accurately

2. **Integration Test**: Use Pyra project as real-world test:
   - Command: `python-inliner pyra.py pyra-inlined.py agent,llm_api,modules,pyra,tools -v`
   - Verify: `./pyra-inlined.py --version` runs without errors
   - Verify: `./pyra-inlined.py` can execute basic commands
   - Verify: Inlined code passes Python syntax check (`python -m py_compile`)

3. **Regression Test**: Verify TYPE_CHECKING fix still works:
   - Run existing TYPE_CHECKING test cases
   - Verify TYPE_CHECKING blocks are still stripped
   - Verify both fixes work together

**Related Issues**:
- This is distinct from TYPE_CHECKING issue (already has TODO item)
- Both issues involve incorrect inlining, but have different root causes
- Both should be fixed and tested together
- Real-world projects (like Pyra) exhibit both issues

**Benefits of Fix**:
- Makes Python Inliner work with virtually all Python projects
- Correctly handles the most common import patterns (module-level imports)
- Maintains semantic correctness of inlined code
- Enables true single-file distribution of multi-module projects
- No more IndentationError on module-level code

**Reference Context**:
- **Project**: Pyra AI Agent
- **Location**: `/Users/billdoughty/src/wdd/python/agents`
- **Command**: `python-inliner pyra.py pyra-inlined.py agent,llm_api,modules,pyra,tools -v`
- **Failing Line**: 1670 in `pyra-inlined.py`
- **Error**: `IndentationError: unexpected indent`
- **Specific Import Chain**: `agent/callbacks.py` → `llm_api/provider_config.py` → `llm_api/environment.py`
- **Output Size**: 533KB (50+ modules inlined)
- **Build Date**: 2025-01-07

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
