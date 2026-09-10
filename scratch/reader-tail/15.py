# test_set.py:1429, a comma value in a class body.
class C:
    cases = 1, 2, 3
print(C.cases[0], C.cases[1], C.cases[2])
