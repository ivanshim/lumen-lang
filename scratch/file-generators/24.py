class Box:
    pass
x = Box()
x.child = Box()
x.child.values = [0, 0]
x.child.values[1] = 7
x.left, x.right = 3, 4
print(x.child.values[1], x.left, x.right)
