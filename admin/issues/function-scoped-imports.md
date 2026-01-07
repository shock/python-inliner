# Function-Scoped Imports: Analysis and Solutions

**Date**: 2025-01-07
**Status**: Design discussion complete, implementation pending
**Priority**: High

---

## Original Problem Statement

From `/Users/billdoughty/src/wdd/rust/python-inliner/TODO.md`:

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

---

## Critical Discovery: Scope Bug with `processed` HashSet

During investigation, a more fundamental issue was discovered:

### Current Algorithm Flaw

The inliner uses a `processed` HashSet to prevent duplicate code inlining:

```rust
// Line 332 in src/main.rs
if processed.insert(init_path.to_path_buf()) {
    // Only inlines if NOT seen before
}
```

### The Bug

When function-level imports appear in multiple functions:

**Input:**
```python
def func1():
    from mylib import helper  # ← First import
    return helper()

def func2():
    from mylib import helper  # ← Second import
    return helper()
```

**Current Output (BROKEN):**
```python
def func1():
    # ↓↓↓ inlined submodule: mylib
    def helper():
        return "hello"
    # ↑↑↑ inlined submodule: mylib
    return helper()  # ✓ helper() is in scope

def func2():
    # →→ mylib ←← module already inlined
    return helper()  # ✗ ERROR! helper() NOT in scope
```

### Runtime Result

```
NameError: name 'helper' is not defined
```

**Critical Issue**: The `processed` HashSet is fundamentally broken for function-level imports because:
- It prevents code duplication (good)
- But also prevents scope sharing (critical bug)
- No way to have both with current architecture

---

## Proposed Solutions

### Solution A: Module-Level Consolidation

**Approach**: Move ALL imports to module level before inlining

#### Algorithm

```
main.py → Phase 1: Scan & Collect → Phase 2: Consolidate → Phase 3: Inline → output.py
         (find all matching   (dedupe by module,    (at top level,
          imports)             preserve order)      not in functions)
```

#### Benefits
- ✓ Module-level constructs stay at module level (where they belong)
- ✓ No code duplication
- ✓ Cleaner output
- ✓ Matches Python's import semantics

#### Drawbacks
- ✗ **Naming conflicts**: If imported module defines same name as file's existing code
  ```python
  def helper():
      return 'original'

  # ↓↓↓ inlined submodule: mylib
  def helper():  # ← CONFLICT! Shadows original
      return 'imported'
  # ↑↑↑ inlined submodule: mylib
  ```
  - Silent behavior changes
  - Original code completely shadowed
  - Dangerous and hard to debug

- ✗ **Import order complexity**: Dependencies must be respected
- ✗ **Insertion point uncertainty**: After shebang? After docstring? After existing imports?
- ✗ **Complexity**: Estimated 2-3 days implementation

---

### Solution B: Inline in Each Function (No Dedup) ⭐ SELECTED

**Approach**: Bypass `processed` set for function-level imports, allow duplication

#### Algorithm

```rust
let indent = cap[1].len();  // Detect indentation level

if indent == 0 {
    // Module-level import
    if processed.insert(module_path) {
        inline_at_module_level();
    } else {
        skip_with_comment();
    }
} else {
    // Function-level import (indent > 0)
    // BYPASS processed set!
    inline_at_function_level();  // Always inline, even if seen before

    if !opt.verbose {
        print_warning();
    }
}
```

#### Warning Message

```
WARNING: Function-scoped import detected at {file}:{line}
         from {module} import {name}

         This import will be inlined into the function body.
         Code duplication will occur if {module} is imported elsewhere.
         Consider moving this import to module level to reduce file size.
```

#### Benefits
- ✓ **Correct scope**: Each function has exactly what it needs
- ✓ **No naming conflicts**: No scope pollution
- ✓ **Preserves semantics**: Maintains original behavior
- ✓ **Simplicity**: Remove `processed` set check for function-level imports
- ✓ **Quick implementation**: Estimated 1-2 hours

#### Drawbacks
- ⚠️ **Code duplication**: Same module inlined multiple times
- ⚠️ **Larger file size**: 3x for module imported in 3 functions

#### Efficiency Analysis

**Question**: Does this approach cause performance issues compared to Python's interpreter?

**Answer**: NO ✓

| Metric                    | Python Runtime | Current Inliner (with HashSet) | Solution B (No Dedup) |
|---------------------------|---------------|-------------------------------|----------------------|
| Module code copies         | 1 (shared)    | N (one per import)           | N (one per import)      |
| Code in output file      | N/A            | 1000 lines                    | 3000 lines (for 3 funcs) |
| Parse time at runtime    | ~10ms          | ~10ms                         | ~10ms                      |
| Memory usage at runtime   | Module object   | Static code                    | Static code                 |
| File size               | N/A            | 10KB                           | 30KB                        |

**Key Insight**:
- Function definitions don't execute until called
- Memory overhead is just code objects, not full source
- File size is cheap, correctness is expensive

**Real-World Impact** (Pyra example):

`llm_api` imported in 3 methods (`call_llm_light`, `call_llm_medium`, `call_llm_heavy`)

If `llm_api` package = 500 lines:
- Python approach: 500 lines in memory
- Solution B: 1500 lines in file
- Overhead: 3x file size

**Is this acceptable?**
- Function-level imports are uncommon in most codebases
- When used, typically for small modules
- Trade-off is pragmatically sound

---

## Comparison: Solution A vs Solution B

| Aspect | Solution A (Consolidation) | Solution B (Inline Each) |
|---------|----------------------------|--------------------------|
| Correctness | ✗ Naming conflicts | ✓ Correct scope |
| Simplicity | ✗ Complex (2-3 days) | ✓ Simple (1-2 hours) |
| File size | ✓ Minimal | ⚠️ Larger (2-3x) |
| Scope safety | ✗ Silent pollution | ✓ No conflicts |
| Implementation risk | ✗ High (edge cases) | ✓ Low |
| Maintains semantics | ✗ May change | ✓ Preserves |

---

## Decision: Solution B Selected

**Rationale**:

1. **Correctness trumps file size**: Broken code is unacceptable
2. **Simplicity wins**: Quick implementation, easy to verify
3. **Pragmatic trade-off**: Function-level imports are the exception, not the rule
4. **User feedback**: Warning messages educate users about the trade-off
5. **Future improvement**: Solution A can be explored as an enhancement

---

## Implementation Plan

### Phase 1: Detection Logic

```rust
let indent = cap[1].len();  // Already captured in regex

if indent == 0 {
    // Module-level import
    if processed.insert(module_path) {
        inline_at_module_level();
    } else {
        skip_with_comment();
    }
} else {
    // Function-level import (indent > 0)
    inline_at_function_level();  // ← BYPASS processed set!

    if !opt.verbose {
        show_function_scoped_warning();
    }
}
```

### Phase 2: Warning Function

```rust
fn show_function_scoped_warning(file: &Path, line_num: usize, module: &str, imports: &str) {
    println!("WARNING: Function-scoped import detected at {}:{line}", file.display(), line_num);
    println!("         from {} import {}", module, imports);
    println!("");
    println!("         This import will be inlined into the function body.");
    println!("         Code duplication will occur if {} is imported elsewhere.", module);
    println!("         Consider moving this import to module level to reduce file size.");
}
```

### Phase 3: Testing

Create test cases covering:
- Function-level import in single function
- Same module imported in multiple functions
- Mixed module-level and function-level imports
- Nested functions with imports
- Class methods with imports
- Warning message verification

---

## Examples

### Example 1: Simple Function-Level Import

**Input:**
```python
def my_function():
    """Process data using mylib."""
    from mylib import helper

    result = helper()
    return result.upper()
```

**Output:**
```python
def my_function():
    """Process data using mylib."""
    # ↓↓↓ inlined submodule: mylib
    def helper():
        return "hello"
    # ↑↑↑ inlined submodule: mylib

    result = helper()
    return result.upper()
```

### Example 2: Duplicate Function-Level Imports

**Input:**
```python
def func1():
    from mylib import helper
    return helper()

def func2():
    from mylib import helper
    return helper()
```

**Output:**
```python
def func1():
    # ↓↓↓ inlined submodule: mylib
    def helper():
        return "hello"
    # ↑↑↑ inlined submodule: mylib
    return helper()

def func2():
    # ↓↓↓ inlined submodule: mylib
    def helper():
        return "hello"
    # ↑↑↑ inlined submodule: mylib
    return helper()
```

**Warning displayed for both imports.**

### Example 3: Mixed Module-Level and Function-Level

**Input:**
```python
from mylib import helper  # Module-level

def func1():
    from mylib import helper  # Function-level
    return helper()

def func2():
    from mylib import helper  # Function-level
    return helper()
```

**Output:**
```python
# ↓↓↓ inlined submodule: mylib  ← At module level
def helper():
    return "hello"
# ↑↑↑ inlined submodule: mylib

def func1():
    # ↓↓↓ inlined submodule: mylib  ← At function level
    def helper():
        return "hello"
    # ↑↑↑ inlined submodule: mylib
    return helper()

def func2():
    # ↓↓↓ inlined submodule: mylib  ← At function level
    def helper():
        return "hello"
    # ↑↑↑ inlined submodule: mylib
    return helper()
```

**Behavior:**
- Module-level import: deduplicated (one inlining)
- Function-level imports: always inline (no deduplication)
- Warnings shown for function-level imports only

---

## Research Tasks

### Existing Python Preprocessors

**Action**: Investigate similar tools to borrow ideas

Potential tools to research:
- [ ] PyInstaller's import analysis
- [ ] Py2exe's module collection
- [ ] Nuitka's optimization
- [ ] Cython's compilation
- [ ] Freeze/pip's bundling

**Questions to answer**:
- How do they handle function-level imports?
- Do they consolidate at module level or preserve structure?
- What strategies do they use for code optimization?
- Are there any academic papers on this problem?

### AST-Based Parsing

**Question**: Should we use Python's `ast` module instead of regex?

**Pros**:
- More robust than regex
- Handles all Python syntax edge cases
- Can parse import statements accurately
- Handles multi-line imports, parenthesized, aliases

**Cons**:
- Requires Python dependency in Rust build
- More complex integration
- Slower than regex for simple cases

**Decision**: Research this for future enhancement, stick with regex for now.

---

## Related Issues

### Issue: Renamed Imports (as keyword)

**Status**: Separate TODO item added

**Problem**: Import statements with aliases are not handled correctly.

**Example**:
```python
from mylib import helper as h

def foo():
    return h()  # Uses alias
```

**Current behavior**: Broken code (NameError)

**Complexity**: Non-trivial (TODO comment in code acknowledges this)

---

## Conclusion

**Recommended Approach**: Solution B (Inline in Each Function)

**Next Steps**:
1. Implement detection logic (indent level check)
2. Bypass `processed` set for function-level imports
3. Add warning messages for function-level imports
4. Create comprehensive test suite
5. Verify against Pyra project
6. Monitor user feedback for potential improvements

**Future Enhancement**: Consider Solution A (module-level consolidation) as optional optimization with `-O` flag.

---

## References

- **Original TODO**: `/Users/billdoughty/src/wdd/rust/python-inliner/TODO.md`
- **Pyra project**: `/Users/billdoughty/src/wdd/python/agents`
- **Source file**: `/Users/billdoughty/src/wdd/python/agents/agent/callbacks.py`
- **Entry point**: `/Users/billdoughty/src/wdd/python/agents/pyra.py`
- **Command used**: `python-inliner pyra.py pyra-inlined.py agent,llm_api,modules,pyra,tools -v`
- **Current code**: `/Users/billdoughty/src/wdd/rust/python-inliner/src/main.rs`

---

**Document version**: 1.0
**Last updated**: 2025-01-07
**Reviewers**: Pending review
