#!/usr/bin/env python
"""
Main test script for python-inliner.

This script tests the inlining functionality with various modules.
It produces deterministic output for verification.
"""

from modules.class1 import Class1
from tacos import Taco
from tacos.hot_sauce import HotSauce
from aliens.alien import Alien
import json


def main():
    """Main entry point for the test script."""
    print("=== Python Inliner Integration Test ===")
    print()

    # Import Class3 from submodules
    from modules.submodules.class3 import Class3

    # Test Class1 (has nested Class2 dependency and JSON usage)
    print("Testing Class1 with nested Class2 dependency:")
    c1 = Class1()
    print(f"  c1.name: {c1.name}")
    print(f"  c1.class2.name: {c1.class2.name}")
    print(f"  c1.data: {c1.data}")
    # Verify the template string is preserved
    print(f"  c1.template (first 50 chars): {c1.template[:50].strip()}...")
    # Verify LONG_DESCRIPTION is preserved
    from modules.class1 import LONG_DESCRIPTION
    print(f"  LONG_DESCRIPTION (first 50 chars): {LONG_DESCRIPTION.strip()[:50]}...")
    print()

    # Test Class3
    print("Testing Class3 from submodules:")
    c3 = Class3()
    print(f"  c3.name: {c3.name}")
    print()

    # Test Taco package
    print("Testing Taco package:")
    taco = Taco("Carnitas")
    print(f"  {taco}")
    print()

    # Test HotSauce from nested package
    print("Testing HotSauce from tacos package:")
    hot_sauce = HotSauce("Habanero")
    print(f"  {hot_sauce}")
    print()

    # Test Alien
    print("Testing Alien:")
    alien = Alien("Zorgon")
    print(f"  {alien}")
    print()

    # Final verification - parse JSON to ensure json module works
    print("Verifying JSON functionality:")
    data = json.loads(c1.data)
    print(f"  Parsed data: {data}")
    print()

    print("=== All Tests Passed ===")


if __name__ == "__main__":
    # Import Class3 again to test duplicate handling
    from modules.submodules.class3 import Class3

    main()
