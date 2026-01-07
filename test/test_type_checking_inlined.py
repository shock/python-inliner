#!/usr/bin/env python
"""Test case for TYPE_CHECKING block handling."""
# ↓↓↓ inlined submodule: modules.module_with_type_checking
"""Module that uses TYPE_CHECKING to avoid runtime imports."""
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # ↓↓↓ inlined submodule: modules.type_only_module
    """Module that should only be imported for type checking."""
    
    class TypeOnlyClass:
        """A class that should only be imported in TYPE_CHECKING blocks."""
        def __init__(self, name: str):
            self.name = name
    
    # ↑↑↑ inlined submodule: modules.type_only_module

def get_message() -> str:
    """Return a simple message."""
    return "Hello from module_with_type_checking"

def process_item(item: "TypeOnlyClass") -> str:
    """Process an item (type hint only, not used at runtime)."""
    return f"Processing: {item.name}"

# ↑↑↑ inlined submodule: modules.module_with_type_checking

def main():
    msg = get_message()
    print(msg)
    # Note: We don't actually use TypeOnlyClass at runtime
    # It's only imported for type checking

if __name__ == "__main__":
    main()
