# tests/python/test_decorators.py:56
if False:
    def call(*args):
        try:
            return saved[args]
        except KeyError:
            res = func(*args)
            saved[args] = res
            return res
        except TypeError:
            # Unhashable argument
            return func(*args)

print('read')
