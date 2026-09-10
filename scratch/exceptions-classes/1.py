class MyError(ValueError): pass
try:
    raise MyError("hello")
except ValueError as e:
    print(type(e).__name__, e.args, str(e))
