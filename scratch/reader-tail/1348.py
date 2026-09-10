# tests/python/test_with.py:45
if False:
    def helper(*args, **kwds):
        return MockContextManager(func, args, kwds)

print('read')
