def f(object):
    object = object + 1
    return object
def g():
    dict = {}
    dict["x"] = 2
    return dict["x"]
print(f(2), g())
