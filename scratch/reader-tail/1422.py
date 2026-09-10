# tests/python/test_decorators.py:12
if False:
        def author(name):
            def decorate(func):
                func.__dict__['author'] = name
                return func
            return decorate

    # -----------------------------------------------


print('read')
