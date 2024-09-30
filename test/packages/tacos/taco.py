class Taco:
    def __init__(self, name):
        self.name = name

    def __str__(self):
        return f"Taco: {self.name}"