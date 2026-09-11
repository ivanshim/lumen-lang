a = ValueError("a")
b = TypeError("b")
try:
    raise a
except ValueError:
    try:
        raise b
    except TypeError:
        try:
            raise a
        except ValueError:
            pass
print(a.__context__ is b)
print(b.__context__ is None)
