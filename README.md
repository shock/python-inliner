# Python Inliner

A high-performance Rust CLI tool that enables modular Python development while supporting single-file distribution. **Primary use case**: Develop using modular repository structures for efficient unit testing and maintenance, then inline modules into a single file for deployment—avoiding the headaches of monolithic file development.

Works with monorepos, cross-repository imports, and third-party source-only packages (no binaries) as long as they're in the Python environment. Recursively inlines Python modules and packages into consolidated files, supporting Python's `sys.path`, editable installs, and advanced import resolution.

## Features

- **🔍 Smart Import Resolution**: Automatically searches Python's `sys.path` for modules
- **📦 Package Support**: Handles packages with `__init__.py` files and nested modules
- **🔄 Circular Import Detection**: Prevents infinite recursion with circular import tracking
- **⚡ Release Mode**: Consolidates imports and removes debug comments for production use
- **🔧 Editable Install Support**: Detects and processes pip editable installations
- **📝 Debug Mode**: Verbose output for debugging complex import chains
- **🎯 Flexible Targeting**: Specify specific modules to inline or default to current directory
- **🛡️ Safe by Design**: Third-party imports are never inlined unless explicitly specified, preventing bloated output

## Installation

### From Source

```bash
git clone https://github.com/shock/python-inliner.git
cd python-inliner
make install
```

### Cargo Install

```bash
cargo install python-inliner
```

## Usage

### Basic Usage

```bash
python-inliner input.py output.py
```

This inlines only modules imported using relative imports (starting with `.`) from the current directory into `input.py` and writes the result to `output.py`.

### Inline Specific Modules

```bash
python-inliner input.py output.py modules,tacos,aliens
```

Only inlines imports from the specified modules (`modules`, `tacos`, `aliens`). The tool will also always inline relative imports (starting with `.`) from the current directory.

### Release Mode

```bash
python-inliner -r input.py output.py
```

Consolidates all imports at the top and removes debug comments for cleaner output.

### Verbose Debugging

```bash
python-inliner -v input.py output.py
```

Shows detailed information about import resolution and processing.

## Examples

*Example code can be found in the `test/` directory of this project.*

### Sample Input

```python
# main.py
from modules.class1 import Class1
from tacos import Taco
from tacos.hot_sauce import HotSauce

def main():
    c1 = Class1()
    taco = Taco("Taco")
    print(taco)

if __name__ == "__main__":
    main()
```

### Basic Inlining

```bash
python-inliner main.py main-inlined.py
```

### Release Mode Inlining

```bash
python-inliner -r main.py main-release.py
```

### Circular Import Example

When circular imports are detected, the tool skips re-inlining and adds comments:

```python
# In the output file:
# →→ modules.class1 ←← module already inlined
```

## Advanced Features

### Recursive Import Handling

The tool recursively inlines imports from inlined modules. When a module is inlined, any imports within that module that match the specified module names will also be inlined.

**Example**:
- `main.py` imports `from modules.class1 import Class1`
- `class1.py` imports `from .class2 import Class2`
- Both `class1.py` and `class2.py` will be inlined into the output

**Safety Feature**: The tool only inlines imports that match the specified module names. Third-party imports within inlined modules are **not** recursively inlined unless explicitly listed. This prevents accidentally inlining large external dependencies and ensures your inlined script remains maintainable.

**Example**:
- `main.py` imports `from modules.class1 import Class1`
- `class1.py` imports `import json` (third-party)
- Only `class1.py` is inlined, `json` remains as an import
- This is intentional - it prevents your inlined script from becoming bloated with external library code

### Circular Import Prevention

The tool maintains a set of processed files to prevent infinite recursion with circular imports. If a module has already been inlined, subsequent imports of the same module will be skipped and marked with comments.

### Default Behavior

When no module names are specified, the tool only inlines **relative imports** (starting with `.`) from the current directory. This means:

- `from . import module` - **will be inlined**
- `from .submodule import func` - **will be inlined**
- `from package import module` - **will NOT be inlined**
- `import sys` - **will NOT be inlined**

To inline specific modules, you must explicitly list them as arguments.

### Python Path Resolution

The tool automatically queries Python's `sys.path` to locate modules, making it compatible with virtual environments and system-wide installations.

### Package Support

Handles complex package structures:
- `from package import module`
- `from package.subpackage import module`
- `import package.module`
- Relative imports (`from . import module`)

### Editable Install Detection

Automatically detects pip editable installations by parsing `direct_url.json` files in `site-packages` directories, ensuring local development packages are properly inlined.

### Import Consolidation

In release mode (`-r`), the tool:
- Collects all imports and places them at the top of the file
- Removes duplicate imports
- Preserves shebang lines
- Maintains proper import ordering

## Use Cases

### Distribution

Create single-file Python scripts for easier distribution without complex module dependencies.

### Deployment

Simplify deployment by bundling all required modules into a single executable script.

### Code Obfuscation

Generate consolidated files that are harder to reverse-engineer (when combined with other tools).

### Testing

Test import resolution and module dependencies in complex Python projects.

## Command Line Reference

```bash
python-inliner [FLAGS] <input-file> <output-file> [module-names]

FLAGS:
    -h, --help       Prints help information
    -r, --release    Suppress comments in the output, and consolidate imports
    -V, --version    Prints version information
    -v, --verbose    Print verbose debug information

ARGS:
    <input-file>      Path to the input Python file
    <output-file>     Path to the output file
    <module-names>    Comma-separated list of module names to inline [default: only relative imports]
```

## Development

### Building

```bash
make debug    # Build debug version
make release  # Build release version (includes tests)
make test     # Run tests
make clean    # Clean build artifacts
```

### Testing

```bash
cargo test
```

## Architecture

Built in Rust for performance and reliability:
- **File System Abstraction**: Trait-based file operations for testing
- **Python Integration**: Subprocess execution for `sys.path` resolution
- **Regex-based Parsing**: Efficient import statement detection
- **Recursive Processing**: Handles nested imports and packages

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) and check the [SPEC.md](SPEC.md) for detailed requirements.
