"""Module that should only be imported for type checking."""

class TypeOnlyClass:
    """A class that should only be imported in TYPE_CHECKING blocks."""
    def __init__(self, name: str):
        self.name = name
