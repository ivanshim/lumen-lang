def make():
    class Local:
        def method(self): return 1
    return Local()
make()
