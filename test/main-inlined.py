#!/usr/bin/env python
# ↓↓↓ inlined submodule: modules.class1

import sys

# ↓↓↓ inlined submodule: .class2
class Class2:
    import sys

    def __init__(self):
        self.name = "Class2"

# ↑↑↑ inlined submodule: .class2

class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()

# ↑↑↑ inlined submodule: modules.class1
# ↓↓↓ inlined package: tacos
# ↓↓↓ inlined submodule: .taco
class Taco:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Taco: {self.name}"
# ↑↑↑ inlined submodule: .taco

__all__ = ["Taco"]
# ↑↑↑ inlined package: tacos

# ↓↓↓ inlined submodule: tacos.hot_sauce
class HotSauce:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"HotSauce: {self.name}"
# ↑↑↑ inlined submodule: tacos.hot_sauce
# ↓↓↓ inlined submodule: aliens.alien
class Alien:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Alien: {self.name}"
# ↑↑↑ inlined submodule: aliens.alien

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