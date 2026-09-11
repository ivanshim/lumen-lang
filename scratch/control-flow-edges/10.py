class Walk:
    def __init__(self):
        self.n = 0
    def __iter__(self):
        return self
    def __next__(self):
        if self.n == 2:
            raise ValueError("walk")
        value = self.n
        self.n += 1
        return value
class Guard:
    def __enter__(self):
        print("enter")
        return self
    def __exit__(self, kind, value, trace):
        print("exit", kind.__name__)
        return False
try:
    with Guard():
        try:
            for value in Walk():
                print(value)
        finally:
            print("finally")
except ValueError:
    print("caught")
