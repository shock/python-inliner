#!/usr/bin/env python
"""Test case for multi-line TYPE_CHECKING imports (the actual bug)."""
# ↓↓↓ inlined submodule: modules.provider_config
"""Provider configuration with TYPE_CHECKING imports."""
from typing import TYPE_CHECKING

def get_provider_name() -> str:
    """Get the provider name."""
    return "LiteLLM Provider"
# ↑↑↑ inlined submodule: modules.provider_config

def main():
    provider = get_provider_name()
    print(f"Provider: {provider}")

if __name__ == "__main__":
    main()
