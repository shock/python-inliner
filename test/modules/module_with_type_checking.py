"""Module that uses TYPE_CHECKING to avoid runtime imports."""
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from modules.type_only_module import TypeOnlyClass

def get_message() -> str:
    """Return a simple message."""
    return "Hello from module_with_type_checking"

def process_item(item: "TypeOnlyClass") -> str:
    """Process an item (type hint only, not used at runtime)."""
    return f"Processing: {item.name}"
