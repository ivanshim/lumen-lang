import _random
import random
class Cold(_random.Random):
    def __init__(self): pass
cold = Cold()
assert 0.0 <= cold.random() < 1.0
class Index:
    def __index__(self): return 13
r = random.Random(21)
assert 0 <= r.getrandbits(Index()) < 8192
observe = False
class Observed(random.Random):
    def __getattribute__(self, name):
        if observe and name == '_stream':
            raise RuntimeError('stream hook')
        return super().__getattribute__(name)
observed = Observed(21)
observe = True
for function in (observed.random, lambda: observed.getrandbits(12)):
    try: function()
    except RuntimeError as error: assert str(error) == 'stream hook'
    else: raise AssertionError('stream lookup bypassed')
for function in (lambda: r.random(1), lambda: r.getrandbits(-1), lambda: r.getrandbits(k=1)):
    try: function()
    except (TypeError, ValueError): pass
    else: raise AssertionError('invalid native call accepted')
print('random descriptors preserve hooks, cold streams and invalid calls')
