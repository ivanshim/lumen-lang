class Replace:
    def __enter__(self):
        return self
    def __exit__(self, kind, value, trace):
        print("exit")
        raise TypeError("exit")
try:
    with Replace():
        raise ValueError("body")
except TypeError as e:
    print(type(e.__context__).__name__)
