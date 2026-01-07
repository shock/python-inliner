#!/usr/bin/env python
"""Test case for TYPE_CHECKING block handling."""
from modules.module_with_type_checking import get_message, process_item

def main():
    msg = get_message()
    print(msg)
    # Note: We don't actually use TypeOnlyClass at runtime
    # It's only imported for type checking

if __name__ == "__main__":
    main()
