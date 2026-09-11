try:
    try:
        raise ValueError("first")
    except ValueError:
        raise TypeError("second")
except TypeError as e:
    print(type(e.__context__).__name__)
try:
    try:
        raise ValueError("first")
    except ValueError:
        raise TypeError("second") from None
except TypeError as e:
    print(type(e.__context__).__name__)
    print(e.__cause__ is None)
    print(e.__suppress_context__)
