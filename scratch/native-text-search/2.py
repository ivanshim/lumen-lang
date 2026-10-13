text = 'alpha/βeta/gamma'
parts = text.split('/')
alias = parts
parts.append('tail')
assert alias == ['alpha', 'βeta', 'gamma', 'tail']
assert text.rsplit('/', True) == ['alpha/βeta', 'gamma']
assert text.split('/', 1 << 100) == ['alpha', 'βeta', 'gamma']
assert ' a\t b  '.split(None, 1) == ['a', 'b  ']
assert text.partition('/') == ('alpha', '/', 'βeta/gamma')
assert text.rpartition('/') == ('alpha/βeta', '/', 'gamma')
assert text.partition(':') == (text, '', '')
assert text.replace('/', ':', 1) == 'alpha:βeta/gamma'
assert ' α '.strip() == 'α'
assert '..α..'.lstrip('.') == 'α..'
assert '..α..'.rstrip('.') == '..α'
assert text.removeprefix('alpha/') == 'βeta/gamma'
assert text.removesuffix('/gamma') == 'alpha/βeta'
assert 'Straße'.casefold() == 'strasse'
assert 'ABC'.lower() == 'abc' and 'abc'.upper() == 'ABC'
class Text(str):
    def split(self, *args, **kwargs): return ['override']
subclass = Text('a/b')
assert subclass.split('/') == ['override']
assert str.split(subclass, '/') == ['a', 'b']
assert text.split(sep='/', maxsplit=1) == ['alpha', 'βeta/gamma']
for function in (lambda: text.split(''), lambda: text.replace('/', None), lambda: text.strip(2)):
    try: function()
    except (TypeError, ValueError): pass
    else: raise AssertionError('invalid native transform accepted')
print('native text transforms preserve mutable results, Unicode and dispatch')
