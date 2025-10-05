
import sys

from .class2 import Class2
# This should NOT be inlined since 'json' is not in the module list
import json

class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()
        # Use json to test if it gets inlined
        self.data = json.dumps({"name": "Class1"})
