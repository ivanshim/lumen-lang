def g():
    try:
        print("must not enter a body whose suspension is unavailable")
        yield 1
    finally:
        print("must not enter its final part either")
next(g())
