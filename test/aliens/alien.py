"""Alien module for testing package imports."""


class Alien:
    """Represents an extraterrestrial being."""

    def __init__(self, name):
        """Initialize alien with a name."""
        self.name = name

    def __str__(self):
        """Return string representation of alien."""
        return f"Alien: {self.name}"
