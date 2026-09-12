print([1, (2, [3])] == [1.0, (2, [3])])
print([1] == (1,), [1] != (1,))
print({'a': [1, (2,)], 'b': {3, 4}} == {'b': {4, 3}, 'a': [1.0, (2,)]})
print({(1, 2), (2, 3)} == {(2, 3), (1, 2)})
d = {(1, 2): 'yes', (2, 3): [4]}
print(d[(1, 2)], (2, 3) in d, (1, 2) in {(1, 2), (3, 4)})
print(None == None, None == 0, None == [], None != '')
try:
    print(None < 1)
except TypeError as e:
    print(str(e))
