def middle():
    print("middle")
    return 2
print(1 < middle() < 3, 3 < 2 < absent())
print(2 == 2 == 2, 2 == 2 == 3, "a" in "abc" in "abcdef")
x = 1
print(x < (x := 2) < 3, x)
a = [1]
b = a
print(a is b, a is [1], 1 == "1", None == False)
print(not 2 in [1], 2 not in [1], None is not None)
