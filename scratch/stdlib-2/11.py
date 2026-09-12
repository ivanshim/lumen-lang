from contextlib import contextmanager
@contextmanager
def manager():
    yield 1
with manager():
    pass
