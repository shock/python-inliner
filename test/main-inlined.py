# !/usr/bin/env python
# ↓↓↓ inlined module: modules.class1

import sys

# ↓↓↓ inlined module: .class2
class Class2:
    import sys

    def __init__(self):
        self.name = "Class2"

# ↑↑↑ inlined module: .class2


class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()

# ↑↑↑ inlined module: modules.class1

# ↓↓↓ inlined package: tacos
# ↓↓↓ inlined module: .taco
class Taco:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Taco: {self.name}"
# ↑↑↑ inlined module: .taco


__all__ = ["Taco"]
# ↑↑↑ inlined package: tacos
# ↓↓↓ inlined module: tacos.Taco
class Taco:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Taco: {self.name}"
# ↑↑↑ inlined module: tacos.Taco

# ↓↓↓ inlined module: aliens.alien
class Alien:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Alien: {self.name}"
# ↑↑↑ inlined module: aliens.alien


def main():
    # ↓↓↓ inlined module: modules.submodules.class3
    class Class3:
        def __init__(self):
            self.name = "Class3"
    # ↑↑↑ inlined module: modules.submodules.class3

    c1 = Class1()
    print(c1.name)
    print(c1.class2.name)
    c3 = Class3()
    print(c3.name)
    taco = Taco("Taco")
    print(taco)
    alien = Alien("Alien")
    print(alien)

if __name__ == "__main__":

    main()