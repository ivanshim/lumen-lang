# The expected output is written for Python 3.14; Python 3.11 cannot parse
# the generic function syntax (introduced in Python 3.12).
class Generic:
    def run(self):
        def __nested[T](__value):
            return __value
        return __nested.__name__, __nested(3)
    def __method[T](self, __value):
        return __value
print(Generic().run())
print(Generic._Generic__method.__name__, Generic()._Generic__method(4))
