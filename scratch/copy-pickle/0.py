import copy
a = [1, [2]]
b = copy.copy(a)
c = copy.deepcopy(a)
a[1].append(3)
print(b, c, copy.copy((1,)) is (1,) or True)
