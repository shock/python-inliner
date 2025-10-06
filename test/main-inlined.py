#!/usr/bin/env python
# ↓↓↓ inlined submodule: modules.class1

import sys

# ↓↓↓ inlined submodule: .class2
class Class2:
    import sys

    def __init__(self):
        self.name = "Class2"

# ↑↑↑ inlined submodule: .class2
# This should NOT be inlined since 'json' is not in the module list
import json

class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()
        # Use json to test if it gets inlined
        self.data = json.dumps({"name": "Class1"})

# ↑↑↑ inlined submodule: modules.class1
from tacos import Taco
from tacos.hot_sauce import HotSauce
from aliens.alien import Alien

def main():
    # ↓↓↓ inlined submodule: modules.submodules.class3
    class Class3:
        def __init__(self):
            self.name = "Class3"
    # ↑↑↑ inlined submodule: modules.submodules.class3
    c1 = Class1()
    print(c1.name)
    print(c1.class2.name)
    c3 = Class3()
    print(c3.name)
    taco = Taco("Taco")
    print(taco)
    alien = Alien("Alien")
    print(alien)
    hot_sauce = HotSauce("HotSauce")
    print(hot_sauce)

if __name__ == "__main__":
    # →→ modules.submodules.class3 ←← module already inlined
    main()