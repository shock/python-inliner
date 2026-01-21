
"""Class1 module for testing inlining functionality."""

import sys

from .class2 import Class2

# This should NOT be inlined since 'json' is not in the module list
import json

# This is a multi-line string assigned to a variable - should NOT be stripped
LONG_DESCRIPTION = """
This is a long description assigned to a variable.
It contains multiple lines and should not be removed
during the docstring stripping phase.
"""


class Class1:
    """First test class with dependency on Class2."""

    def __init__(self):
        """Initialize Class1 with a name and Class2 instance."""
        self.name = "Class1"

        # Create Class2 instance
        self.class2 = Class2()

        # Use json to test if it gets inlined
        self.data = json.dumps({"name": "Class1"})

        # Another multi-line string assigned to a variable
        self.template = """
        This is a template string assigned to self.template.
        It should also be preserved during docstring stripping.
        """

        # F-string with interpolation and multi-line - should be preserved
        some_var = f"""long
string {self.name} with interpolation
"""



