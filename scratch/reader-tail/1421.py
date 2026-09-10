# tests/python/test_decorators.py:5
if False:
    def decorate(func):
        func.__dict__.update(kwds)
        return func

print('read')
