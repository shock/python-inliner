# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`python-inliner` is a Rust CLI tool that inlines imported Python modules from a specified directory into a given Python script, creating a single consolidated file. It handles both modules and packages with `__init__.py` files, and can process imports from Python's `sys.path`.

## Development Commands

### Building and Testing
- `make debug` - Build debug version
- `make release` - Build release version (includes running tests)
- `make test` - Run tests and generate test output file
- `make clean` - Clean build artifacts
- `make install` - Install to `/opt/local/bin` (default target)

### Cargo Commands
- `cargo build` - Build debug version
- `cargo build --release` - Build release version
- `cargo test` - Run unit tests
- `cargo run test/main.py test/main-inlined.py tacos,modules` - Run with test data

## Architecture

### Core Components

**Main Entry Point** (`src/main.rs`):
- CLI argument parsing using `structopt`
- Main application logic with recursive import inlining
- Handles Python `sys.path` resolution and editable installs
- Post-processing for release mode (consolidates imports)

**File System Abstraction** (`src/modules/file_system.rs`):
- `FileSystem` trait defining file operations
- `RealFileSystem` implementation using std::fs
- `VirtualFileSystem` (in `virtual_filesystem.rs`) for testing

**Python Integration** (`src/utils/python.rs`):
- `get_python_sys_path()` - Executes Python to get system path
- Handles Python subprocess execution and error handling

### Key Features

1. **Import Detection**: Uses regex to find `from module import ...` statements
2. **Recursive Inlining**: Processes modules and packages recursively
3. **Path Resolution**: Searches Python's `sys.path` and handles relative imports
4. **Package Support**: Handles `__init__.py` files and package structures
5. **Editable Installs**: Detects and processes editable pip installs via `direct_url.json`
6. **Release Mode**: Option to consolidate imports and remove debug comments

### Testing Strategy

- Unit tests in `main.rs` using `VirtualFileSystem` for isolation
- Integration tests with real Python files in `test/` directory
- Tests cover basic inlining, circular import detection, and import consolidation

## Usage

The tool takes three main arguments:
```bash
python-inliner <input_file> <output_file> [module_names]
```

- `input_file`: Python script with imports
- `output_file`: Where to write the inlined result
- `module_names`: Comma-separated list of module names to inline (default: current directory modules)

Optional flags:
- `-r, --release`: Consolidate imports and remove debug comments
- `-v, --verbose`: Print debug information

## Code Conventions

Based on `ENGINEERING_GUIDELINES.md`:
- Keep functions small and focused with clear names
- Use descriptive variable names and comments for complex logic
- Follow Rust best practices and conventions
- Write unit tests for all functions
- Use integration tests for end-to-end scenarios

## File Structure

- `src/main.rs` - Main application logic and CLI
- `src/modules/` - File system abstractions
- `src/utils/` - Utility functions (Python integration)
- `test/` - Test Python files and modules
- `llm/` - Design specifications and planning documents