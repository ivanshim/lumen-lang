# tests/python/test_decorators.py:42
if False:
    def decorate(func):
        func_name = func.__name__
        counts[func_name] = 0
        def call(*args, **kwds):
            counts[func_name] += 1
            return func(*args, **kwds)
        call.__name__ = func_name
        return call

print('read')
