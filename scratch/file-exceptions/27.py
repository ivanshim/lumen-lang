def base():
    print("base must wait")
    return object

class Example(base()):
    pass
