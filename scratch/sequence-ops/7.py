print([1, 2, 3][-1], (4, 5)[-2], 'abc'[-2])
try:
    print([1][-2])
except IndexError as e:
    print(str(e))
try:
    print((1,)[1])
except IndexError as e:
    print(str(e))
try:
    print('a'[1])
except IndexError as e:
    print(str(e))
t = (1, 2)
s = 'ab'
try:
    t[0] = 9
except TypeError as e:
    print(str(e))
try:
    del t[0]
except TypeError as e:
    print(str(e))
try:
    s[0] = 'z'
except TypeError as e:
    print(str(e))
try:
    del s[0]
except TypeError as e:
    print(str(e))
