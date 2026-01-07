# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`python-inliner` is a Rust CLI tool that inlines imported Python modules into a single consolidated file. Primary use case: develop with modular structure for testing/maintenance, then inline for single-file deployment.

Supports: `sys.path` resolution, editable installs, packages with `__init__.py`, circular import detection, and release mode (consolidated imports, no debug comments).

## Development Commands

### Building and Testing
- `make debug` - Build debug version
- `make release` - Build release version (runs tests first)
- `make test` - Run integration test with test/ files and unit tests
- `cargo test` - Run unit tests only
- `cargo test <test_name>` - Run specific test
- `cargo run -- test/main.py test/output.py modules,tacos` - Test with example files
- `cargo run -- -v <input> <output> <modules>` - Run with verbose debug output

### Installation
- `make install` - Install to `/opt/local/bin` (builds release first)

## Architecture

### Core Components

**Main Entry Point** (`src/main.rs`):
- CLI parsing with `structopt`
- `run()` - Main orchestration: resolves paths, processes module names, calls `inline_imports()`
- `inline_imports()` - Recursive core: regex-based import detection, path resolution, content inlining
- `post_process_imports()` - Release mode: consolidate imports at top, remove debug comments
- `handle_editable_installs()` - Parse `direct_url.json` to find pip editable installs

**File System Abstraction** (`src/modules/file_system.rs`):
- `FileSystem` trait - Abstraction for file operations (read, write, exists, is_dir, etc.)
- `RealFileSystem` - Production implementation using `std::fs`
- `VirtualFileSystem` (`virtual_filesystem.rs`) - In-memory implementation for unit tests

**Python Integration** (`src/utils/python.rs`):
- `get_python_sys_path()` - Executes `python -c "import sys; print(sys.path)"` to get search paths

### Import Inlining Algorithm

1. **Regex matching**: Finds `from <module> import <items>` statements matching specified module names
2. **Path resolution**: For each import, searches `python_sys_path` for:
   - Package: `<module>/__init__.py`
   - Module: `<module>.py`
   - Relative imports (`.module`): resolved relative to current file's parent directory
3. **Recursive processing**: Inlined content is recursively processed for more imports
4. **Circular detection**: `processed: HashSet<PathBuf>` tracks inlined files to prevent infinite recursion
5. **Content replacement**: Import statements replaced with inlined module content (indented to match import line)

### Module Name Matching

- **Always inlines**: Relative imports (`from .module import ...`) - automatically prepended to module list as `"\\."`
- **User-specified**: Comma-separated module names become regex pattern: `(module1|module2|module3)`
- **Never inlines** (unless explicitly listed): Third-party imports, standard library

### Testing Strategy

**Unit Tests** (`src/main.rs` `#[cfg(test)]`):
- Use `VirtualFileSystem` to create isolated in-memory file structures
- Test scenarios: basic inlining, circular imports, import consolidation
- No file I/O required - all tests run against virtual filesystem

**Integration Test** (`make test`):
- Runs actual tool against `test/main.py` with real Python modules in `test/modules/` and `test/tacos/`
- Compares output to expected behavior
- Validates with `cargo test` (which includes integration test verification)

## Key Behaviors

### Release Mode (`-r` flag)
- Collects all `import` and `from ... import` statements from inlined content
- Moves them to top of file (after shebang if present)
- Removes duplicate imports
- Strips all debug comments (`# ↓↓↓ inlined ...`, `# ↑↑↑ inlined ...`, `# →→ ... already inlined`)

### Circular Import Handling
- First occurrence: Module is inlined normally
- Subsequent occurrences: Import statement replaced with comment `# →→ <module> ←← already inlined`
- In release mode: Import line is removed entirely (no comment)

### Indentation Preservation
- Inlined content inherits indentation of the import statement it replaces
- All newlines in inlined content get additional indentation: `.replace("\n", &format!("\n{indent}"))`
- Enables inlining imports inside functions, class definitions, etc.

## Known Issues / TODO

Check `TODO.md` for current issues. Notable issue:

**TYPE_CHECKING Block Handling** (TODO.md): Currently inlines imports from `if TYPE_CHECKING:` blocks incorrectly, causing indentation errors. These blocks should be preserved as-is since they're only for static type checkers.

## Code Conventions (from ENGINEERING_GUIDELINES.md)

- Small, focused functions with clear names
- Verbose variable names and comments for complex logic
- Each function should have unit tests
- Integration tests cover real-world scenarios
