def g():
    try:
        yield 1
        return 7
    finally:
        print("closed")
g()
