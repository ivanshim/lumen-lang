text = 'aébéa'
assert text.find('é') == 1
assert text.rfind('é') == 3
assert text.index('é', 2) == 3
assert text.rindex('é', 0, 3) == 1
assert text.count('é') == 2
assert text.startswith('éb', 1, 4)
assert text.endswith('bé', 1, 4)
assert text.find('é', -3) == 3
assert text.find('é', None, None) == 1
assert text.find('é', True) == 1
assert text.find('', 20) == -1
assert not text.startswith('', 20)
assert text.startswith(('z', 'a'))
assert text.find('é', 10**100) == -1
assert text.find('é', -(10**100)) == 1
class Text(str):
    def find(self, *args): return 37
assert Text(text).find('é') == 37
assert str.find(Text(text), 'é') == 1
assert text.find(Text('é')) == 1
assert '\ud800a'.find('a') == 1
for method in (text.index, text.rindex):
    try: method('z')
    except ValueError: pass
    else: raise AssertionError('missing substring did not raise')
for call in (lambda: text.find(), lambda: text.find(b'a'),
             lambda: text.find('a', 0, 2, 3), lambda: text.find('a', start=0), lambda: text.find('a', object())):
    try: call()
    except TypeError: pass
    else: raise AssertionError('invalid search did not raise')
print('native searches preserve bounds, errors and subclass dispatch')
