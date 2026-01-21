"""Taco module for testing package imports."""


class Taco:
    """Represents a delicious taco."""

    def __init__(self, name):
        """Initialize taco with a name."""
        self.name = name

    def __str__(self):
        """Return string representation of taco."""
        return f"Taco: {self.name}"
