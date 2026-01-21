# Python Inliner

A high-performance Rust CLI tool that enables modular Python development while supporting single-file distribution. **Primary use case**: Develop using modular repository structures for efficient unit testing and maintenance, then inline modules into a single file for deployment—avoiding the headaches of monolithic file development.

Works with monorepos, cross-repository imports, and third-party source-only packages (no binaries) as long as they're in the Python environment. Recursively inlines Python modules and packages into consolidated files, supporting Python's `sys.path`, editable installs, and advanced import resolution.

## Features

- **🔍 Smart Import Resolution**: Automatically searches Python's `sys.path` for modules
- **📦 Package Support**: Handles packages with `__init__.py` files and nested modules
- **🔄 Circular Import Detection**: Prevents infinite recursion with circular import tracking
- **⚡ Release Mode**: Production-ready output with consolidated imports, stripped docstrings/comments, and minimized file size
- **🔧 Editable Install Support**: Detects and processes pip editable installations
- **📝 Debug Mode**: Verbose output for debugging complex import chains
- **🎯 Flexible Targeting**: Specify specific modules to inline or default to current directory
- **🛡️ Safe by Design**: Third-party imports are never inlined unless explicitly specified, preventing bloated output

## Safety Features

**🔒 Third-party Import Protection**: External libraries are never inlined unless explicitly specified, preventing accidental inclusion of large dependencies and maintaining clean output.

**🛡️ Circular Import Prevention**: Built-in tracking prevents infinite recursion and handles complex import chains gracefully.

**🎯 Explicit Control**: Only processes modules you explicitly list, giving you complete control over what gets inlined.

## Installation

```bash
# Clone and build
git clone https://github.com/shock/python-inliner.git
cd python-inliner
# Create /opt/local/bin and add to PATH, if needed
curl -sSL https://raw.githubusercontent.com/shock/string_space/refs/heads/master/setup_opt_local_bin.sh | /bin/bash
make install

# Try the example
python-inliner test/main.py test/output.py modules,tacos
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

Produces production-ready, minimized output with:
- **Import consolidation**: All imports moved to the top, duplicates removed
- **Docstring removal**: Function and class docstrings stripped (preserves variable assignments and f-strings)
- **Comment removal**: All comments removed (preserves shebang lines)
- **Blank line removal**: All unnecessary whitespace eliminated
- **Debug comment removal**: Inlining markers (↓↓↓, ↑↑↑, →→) stripped

Perfect for deployment where smaller file size and IP protection are priorities.

### Verbose Debugging

```bash
python-inliner -v input.py output.py
```

Shows detailed information about import resolution and processing.

## Examples

*Example code can be found in the `test/` directory of this project.*

### Example: Before and After

**Input File** (`main.py`):
```python
#!/usr/bin/env python
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

**Output File** (`main-inlined.py`):
```python
#!/usr/bin/env python
# ↓↓↓ inlined submodule: modules.class1

# ↓↓↓ inlined submodule: .class2
class Class2:
    def __init__(self):
        self.name = "Class2"
# ↑↑↑ inlined submodule: .class2

class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()
# ↑↑↑ inlined submodule: modules.class1

# ↓↓↓ inlined package: tacos
# ↓↓↓ inlined submodule: .taco
class Taco:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Taco: {self.name}"
# ↑↑↑ inlined submodule: .taco

__all__ = ["Taco"]
# ↑↑↑ inlined package: tacos

# ↓↓↓ inlined submodule: tacos.hot_sauce
class HotSauce:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"HotSauce: {self.name}"
# ↑↑↑ inlined submodule: tacos.hot_sauce

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

### Release Mode Processing

Release mode (`-r`) applies a series of optimizations to produce production-ready output:

**1. Import Consolidation**
- Collects all imports and places them at the top of the file (after shebang)
- Removes duplicate imports
- Maintains proper import ordering

**2. Docstring Removal**
- Strips all module, class, and function docstrings
- Preserves triple-quoted strings assigned to variables (e.g., `TEMPLATE = """..."""`)
- Preserves f-strings (e.g., `f"""string {var}"""`)
- Ideal for IP protection while maintaining functionality

**3. Comment Removal**
- Removes all inline and whole-line comments
- Preserves shebang lines (`#!/usr/bin/env python3`)
- Handles comments inside strings correctly

**4. Blank Line Removal**
- Eliminates all blank lines (including whitespace-only lines)
- Produces compact, minimal output

**5. Debug Marker Removal**
- Strips inlining markers (↓↓↓, ↑↑↑, →→)
- Creates clean production code

## Use Cases

### Distribution

Create single-file Python scripts for easier distribution without complex module dependencies.

### Deployment

Simplify deployment by bundling all required modules into a single executable script.

### IP Protection

Release mode removes all documentation and comments, making code harder to understand and reverse-engineer while maintaining full functionality.

### Testing

Test import resolution and module dependencies in complex Python projects.

## Troubleshooting

### Module Not Found
- Ensure the module is in Python's `sys.path` or current directory
- Check virtual environment activation
- Verify module names are spelled correctly

### Circular Imports
The tool automatically detects and handles circular imports by skipping re-inlining and adding comments.

### Third-party Libraries
Third-party imports are never inlined by default to prevent bloated output. Only explicitly listed modules are processed.

### Performance Issues
For very large projects, consider inlining specific modules rather than all modules to reduce processing time.

## Command Line Reference

```bash
python-inliner [FLAGS] <input-file> <output-file> [module-names]

FLAGS:
    -h, --help       Prints help information
    -r, --release    Production mode: consolidate imports, strip docstrings/comments/blank lines
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

## Performance

Built in Rust for exceptional performance:
- **Fast Import Resolution**: Regex-based parsing for efficient import detection
- **Minimal Memory Footprint**: Efficient recursive processing
- **Quick Execution**: Processes complex import chains in milliseconds

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please read our [Contributing Guidelines](CONTRIBUTING.md) and check the [SPEC.md](SPEC.md) for detailed requirements.
