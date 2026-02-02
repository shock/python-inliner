# Quick Reference for Pyra AI Agent

## Release Mode Behavior
- Flag: `-r` or `--release`
- Search: `if release` in `src/main.rs`
- Operations (applied in order):
  1. `post_process_imports()` - consolidates and sorts imports
  2. `strip_docstrings()` - removes docstrings, preserves triple-quoted variable assignments
  3. `strip_comments()` - removes comments, preserves shebang and `#` in strings
  4. `strip_blank_lines()` - removes all blank lines

## Key Functions to Search
- `strip_docstrings` - docstring removal logic
- `strip_comments` - comment removal logic
- `strip_blank_lines` - blank line removal logic
- `fn run` - main orchestrator
- `fn inline_imports` - core inlining logic

## Testing
- Search: `test_release_mode_complete_flow` - integration test for full release mode flow
