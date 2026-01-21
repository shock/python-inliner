#!/usr/bin/env python
import json
import sys
class Class2:
    def __init__(self):
        self.name = "Class2"
LONG_DESCRIPTION = """
This is a long description assigned to a variable.
It contains multiple lines and should not be removed
during the docstring stripping phase.
"""
class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()
        self.data = json.dumps({"name": "Class1"})
        self.template = """
        This is a template string assigned to self.template.
        It should also be preserved during docstring stripping.
        """
        some_var = f"""long
string {self.name} with interpolation
"""
class Taco:
    def __init__(self, name):
        self.name = name
    def __str__(self):
        return f"Taco: {self.name}"
__all__ = ["Taco"]
class HotSauce:
    def __init__(self, name):
        self.name = name
    def __str__(self):
        return f"HotSauce: {self.name}"
class Alien:
    def __init__(self, name):
        self.name = name
    def __str__(self):
        return f"Alien: {self.name}"
def main():
    print("=== Python Inliner Integration Test ===")
    print()
    class Class3:
        def __init__(self):
            self.name = "Class3"
    print("Testing Class1 with nested Class2 dependency:")
    c1 = Class1()
    print(f"  c1.name: {c1.name}")
    print(f"  c1.class2.name: {c1.class2.name}")
    print(f"  c1.data: {c1.data}")
    print(f"  c1.template (first 50 chars): {c1.template[:50].strip()}...")
    print(f"  LONG_DESCRIPTION (first 50 chars): {LONG_DESCRIPTION.strip()[:50]}...")
    print()
    print("Testing Class3 from submodules:")
    c3 = Class3()
    print(f"  c3.name: {c3.name}")
    print()
    print("Testing Taco package:")
    taco = Taco("Carnitas")
    print(f"  {taco}")
    print()
    print("Testing HotSauce from tacos package:")
    hot_sauce = HotSauce("Habanero")
    print(f"  {hot_sauce}")
    print()
    print("Testing Alien:")
    alien = Alien("Zorgon")
    print(f"  {alien}")
    print()
    print("Verifying JSON functionality:")
    data = json.loads(c1.data)
    print(f"  Parsed data: {data}")
    print()
    print("=== All Tests Passed ===")
if __name__ == "__main__":
    main()
