"""HotSauce module for testing nested package imports."""


class HotSauce:
    """Represents spicy hot sauce."""

    def __init__(self, name):
        """Initialize hot sauce with a name."""
        self.name = name

    def __str__(self):
        """Return string representation of hot sauce."""
        return f"HotSauce: {self.name}"
