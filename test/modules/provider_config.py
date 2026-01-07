"""Provider configuration with TYPE_CHECKING imports."""
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from modules.environment import (
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
