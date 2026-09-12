class Manager:
    def __init__(self, name):
        self.name = name
    def __enter__(self):
        print("enter", self.name)
        return self
    def __exit__(self, kind, value, trace):
        print("exit", self.name, type(kind).__name__, type(value).__name__, type(trace).__name__)
        return True
with Manager("error"):
    raise ValueError("oops")
with Manager("normal"):
    print("body")
def f():
    with Manager("return"):
        return 7
print(f())
with Manager("a"), Manager("b"):
    print("both")
