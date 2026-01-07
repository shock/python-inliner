#!/usr/bin/env python
"""Test case for multi-line TYPE_CHECKING imports (the actual bug)."""
from modules.provider_config import get_provider_name

def main():
    provider = get_provider_name()
    print(f"Provider: {provider}")

if __name__ == "__main__":
    main()
