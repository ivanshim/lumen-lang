class Same(ValueError): pass
old = Same("old")
class Same(ValueError): pass
try:
    raise old
except Same:
    print("wrong")
except ValueError as e:
    print("old class", e.args)
class Child(Same): pass
try:
    raise Child
except Same:
    print("new child")
