def middle():
    print("middle")
    return None
print(None is middle() is None)
print(False is None is missing())
print(1 < 2 < 3, 3 < 2 < missing())
print(1 < 2 == 2, 2 == 2 < 3)
print(None is not False is not None)
def mark(name, value):
    print(name)
    return value
print(mark("left", 1) < mark("center", 2) < mark("right", 3))
