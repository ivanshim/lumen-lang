# tests/python/test_decorators.py:29
if False:
    def decorate(func):
        expr = compile(exprstr, "dbcheck-%s" % func.__name__, "eval")
        def check(*args, **kwds):
            if not eval(expr, globals, locals):
                raise DbcheckError(exprstr, func, args, kwds)
            return func(*args, **kwds)
        return check

print('read')
