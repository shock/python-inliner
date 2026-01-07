#!/usr/bin/env python
"""Test case for multi-line TYPE_CHECKING imports (the actual bug)."""
# ↓↓↓ inlined submodule: modules.provider_config
"""Provider configuration with TYPE_CHECKING imports."""
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    # ↓↓↓ inlined submodule: modules.environment
    """Environment configuration variables."""
    
    LITELLM_API_KEY = "litellm-key-123"
    OPENAI_API_KEY = "openai-key-456"
    DEEPSEEK_API_KEY = "deepseek-key-789"
    ZAI_API_KEY = "zai-key-abc"
    XAI_API_KEY = "xai-key-def"
    GEMINI_API_KEY = "gemini-key-ghi"
    
    # ↑↑↑ inlined submodule: modules.environment
        LITELLM_API_KEY,
        OPENAI_API_KEY,
        DEEPSEEK_API_KEY,
        ZAI_API_KEY,
        XAI_API_KEY,
        GEMINI_API_KEY,
    )

def get_provider_name() -> str:
    """Get the provider name."""
    return "LiteLLM Provider"

# ↑↑↑ inlined submodule: modules.provider_config

def main():
    provider = get_provider_name()
    print(f"Provider: {provider}")

if __name__ == "__main__":
    main()
