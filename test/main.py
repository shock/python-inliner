#!/usr/bin/env python
from modules.class1 import Class1
from tacos.taco import Taco
from aliens.alien import Alien

def main():
    from modules.submodules.class3 import Class3
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
    from modules.submodules.class3 import Class3
    main()