# Python-Inliner Release Mode Cleanup - Implementation Plan

## Objective

Implement a final cleanup pass for release mode that produces production-ready Python code by removing non-essential content while preserving all functional code.

### Goals
1. **Minimize file size** - Remove documentation and comments
2. **Protect intellectual property** - Remove docstrings (often contain sensitive info)
3. **Clean formatting** - Remove unnecessary blank lines
4. **Maintain functionality** - All code must work identically to non-release mode

## Requirements

### What to Strip
1. **Blank lines**
   - Single blank lines
   - Multiple consecutive blank lines
   - Lines with only whitespace

2. **Comments**
   - Whole-line comments: `# This is a comment`
   - Inline comments: `x = 1  # This is a comment`
   - Preserve: Comments inside strings (e.g., `"This # is not a comment"`)

3. **Docstrings**
   - Module-level docstrings: `"""Module description."""`
   - Function docstrings: `def func(): """Function docs."""`
   - Class docstrings: `class MyClass: """Class docs."""`
   - Preserve: Variable assignments: `MY_VAR = """Keep this string."""`
   - Preserve: F-strings: `f"""Keep {this} too."""`

### What to Preserve
- ✅ **Shebang lines** (only on first line): `#!/usr/bin/env python3`
- ✅ **Variable assignments** with triple-quoted strings
- ✅ **F-strings** with triple quotes
- ✅ **Triple-quoted strings after imports**: `from x import y """string"""`
- ✅ **Triple-quoted strings after decorators**: `@decorator """string"""`
- ✅ **Comments inside string literals** (both single and triple-quoted)
- ✅ **All functional code** (imports, functions, classes, logic)

## Implementation Plan

### Architecture: Three Separate Passes

**Why separate passes?**
- Easier testing (each function can be unit tested independently)
- Simpler logic (each function has single responsibility)
- Better error debugging (can isolate which pass has issues)
- More maintainable (clear separation of concerns)

### Pass 1: `strip_docstrings(content: &str) -> String`

**Purpose**: Remove function/class docstrings while preserving variable assignments

**Implementation Approach**: Character-based scanner

**Algorithm**:
```
1. Initialize: result = empty, last_copied = 0, i = 0
2. For each character in content:
   a. Detect opening triple quote ('"""' or ''')
   b. Find matching closing triple quote
   c. Get line content before the string
   d. Check if should preserve:
      - Assignment pattern: `^\s*[a-zA-Z_]\w*\s*=`
      - Import pattern: `^\s*(from|import)\s+`
      - Decorator pattern: `^\s*@`
      - F-string: line ends with 'f'
   e. If preserve: copy from last_copied to end of string
   f. If skip: copy from last_copied to start of string
   g. Update last_copied and continue
3. Copy remaining content from last_copied to end
```

**Preservation Patterns** (Regex):
- `r"^\s*[a-zA-Z_]\w*\s*="` - Variable assignments
- `r"^\s*(from|import)\s+"` - Import statements
- `r"^\s*@"` - Decorators
- Check if trimmed line ends with 'f' for F-strings

### Pass 2: `strip_comments(content: &str) -> String`

**Purpose**: Remove all comments while preserving comments inside strings

**Implementation Approach**: Line-by-line with character scanning

**Algorithm**:
```
1. For each line:
   a. Check if shebang (first line only) - preserve as-is
   b. Scan characters to find '#':
      - Track if inside string (", ', """, ')
      - Track triple-quote mode for proper string detection
      - When '#' found outside string: mark position
   c. If '#' found:
      - If line before '#' is blank: skip entire line
      - Else: copy line up to '#', trim trailing whitespace
   d. Else: copy entire line
```

**String Detection Logic**:
- Track: `in_string = None | '"' | '''`
- On quote:
  - Single quote: toggle if not in triple quote
  - Triple quote: if same char, check next 2 chars
- Only remove '#' when `in_string is None`

### Pass 3: `strip_blank_lines(content: &str) -> String`

**Purpose**: Remove all blank lines

**Implementation Approach**: Filter lines

**Algorithm**:
```
1. Split content into lines
2. For each line:
   a. Trim whitespace
   b. If not empty: keep line
   c. If empty: skip line
3. Join kept lines with newlines
4. Preserve final newline if original had it
```

## Integration into Release Mode

### Current Flow (Debug Mode)
```
inlined_content → post_process_imports() → output
```

### New Flow (Release Mode)
```
inlined_content → post_process_imports() → strip_docstrings() → strip_comments() → strip_blank_lines() → output
```

### Location in Code
**File**: `src/main.rs`
**Function**: `run()`
**Lines**: Around 113-118

**Code**:
```rust
let mut content = inline_imports(fs, &python_sys_path, &input_file, &module_names, &mut HashSet::new(), &opt)?;
if release {
    content = post_process_imports(&content);
    content = strip_docstrings(&content);
    content = strip_comments(&content);
    content = strip_blank_lines(&content);
}
fs.write(&output_file, content)?;
```

## Testing Strategy

### Unit Tests (in `src/main.rs`)

#### For `strip_docstrings()`:
1. **Simple docstrings**: Module, function, class docstrings removed
2. **Preserve variable assignment**: `MY_VAR = """text"""` kept
3. **Preserve F-string**: `f"""text {var}"""` kept
4. **Single quotes**: `'''text'''` also handled
5. **No docstrings**: Code without docstrings unchanged
6. **Multiple docstrings**: All removed correctly

#### For `strip_comments()`:
1. **Whole-line comments**: `# comment` removed
2. **Inline comments**: `code  # comment` → `code`
3. **Preserve shebang**: First line `#!/...` kept
4. **Preserve strings with '#'**: `"text # here"` kept
5. **Preserve triple-quoted strings**: `"""text # here"""` kept
6. **No comments**: Code unchanged

#### For `strip_blank_lines()`:
1. **Single blank lines**: Removed
2. **Multiple blank lines**: All removed
3. **No blank lines**: Code unchanged
4. **Whitespace-only lines**: Removed
5. **Preserve final newline**: If input ends with newline

#### Integration Test:
**Test**: `test_release_mode_complete_flow`

**Setup**:
- Mock filesystem with `mylib` package
- Package contains: docstrings, comments, blank lines, variable assignments

**Expected Behavior**:
1. All modules inlined
2. Imports consolidated and sorted
3. All docstrings removed
4. All comments removed
5. All blank lines removed
6. Shebang preserved
7. Variable assignments with triple quotes preserved
8. F-strings preserved
9. Code remains functional

### Real-World Test Files

Updated all test files with realistic content:
- `test/main.py` - Module and function docstrings, comments, blank lines
- `test/modules/class1.py` - Docstrings, comments, variable assignments, F-strings
- `test/modules/class2.py` - Docstrings, comments
- `test/modules/submodules/class3.py` - Docstrings, comments
- `test/packages/tacos/taco.py` - Docstrings, comments
- `test/packages/tacos/hot_sauce.py` - Docstrings, comments
- `test/aliens/alien.py` - Docstrings, comments

## Current Status

### ✅ Completed
1. All three functions implemented
2. Integrated into release mode flow
3. All test files updated with realistic scenarios
4. 16/23 unit tests passing

### ⚠️ Known Issues
1. **`strip_docstrings()` has a critical bug**
   - Character scanner producing mangled output
   - Logic for tracking `last_copied` position is incorrect
   - Test failures show strings being concatenated incorrectly

2. **Test failures** (6/23):
   - 4 docstring tests failing due to `strip_docstrings()` bug
   - 1 comment test failing (`test_strip_comments_preserves_triple_quoted_strings`)
   - 1 integration test failing due to docstring issues

### 🔧 Next Steps

1. **Fix `strip_docstrings()` function**
   - Debug character scanner logic
   - Fix `last_copied` tracking
   - Ensure correct range copying
   - Test all docstring cases

2. **Fix comment test failure**
   - Investigate `test_strip_comments_preserves_triple_quoted_strings`
   - Likely edge case in string detection

3. **Verify integration test**
   - Ensure all three passes work together correctly
   - Verify final output is valid Python
   - Check shebang preservation

## Success Criteria

### When Implementation is Complete:
- ✅ All 23 unit tests pass (1 intentionally ignored)
- ✅ Integration test passes
- ✅ Real-world test files produce valid, clean Python
- ✅ Release mode produces smaller, production-ready files
- ✅ All functional code preserved and executable
- ✅ No documentation leaks (docstrings removed)
- ✅ Clean formatting (no unnecessary blank lines)
- ✅ No comments (except shebang)

### Performance Considerations
- All passes should be O(n) where n = content length
- Memory usage: O(n) for building result strings
- Should complete in < 100ms for typical 10KB Python file
- Should handle large files (> 1MB) without issues

### Edge Cases to Handle
1. **Nested strings**: Strings within docstrings, comments within strings
2. **Escaped quotes**: `\"` within strings (Python handles this)
3. **Mixed quote types**: Single quotes within double-quoted strings and vice versa
4. **Unicode content**: Non-ASCII characters in docstrings/comments
5. **Windows line endings**: `\r\n` vs `\n`
6. **Empty files**: Files with only docstrings/comments
7. **Files with no shebang**: First line is not a shebang
