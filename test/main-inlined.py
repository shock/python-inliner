# ↓↓↓ inlined module: modules.class1
# ↓↓↓ inlined module: modules.class2
class Class2:
    def __init__(self):
        self.name = "Class2"

# ↑↑↑ inlined module: modules.class2


class Class1:
    def __init__(self):
        self.name = "Class1"
        self.class2 = Class2()

# ↑↑↑ inlined module: modules.class1

# ↓↓↓ inlined module: modules.submodules.class3
class Class3:
    def __init__(self):
        self.name = "Class3"
# ↑↑↑ inlined module: modules.submodules.class3


def main():
    c1 = Class1()
    print(c1.name)
    print(c1.class2.name)
    c3 = Class3()
    print(c3.name)

if __name__ == "__main__":
    main()