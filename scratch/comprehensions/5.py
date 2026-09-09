x = 90
print([x + y for x, y in [[1, 2], [3, 4]]])
print([x for x, in [[4], [5]]], [a + b for (a, b) in [[6, 7]]])
print([[x * y for y in [1, 2]] for x in [3, 4]], x)
print([x for x in range(10) if x % 2 if x % 3])
