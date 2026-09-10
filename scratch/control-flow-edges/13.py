def cause():
    print("cause")
    return ValueError("cause")
try:
    raise TypeError("body") from cause()
except Exception as e:
    print(type(e.__cause__).__name__)
    print(e.__context__ is None)
    print(e.__suppress_context__)
