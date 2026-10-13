import math
import random
r = random.Random(1234)
assert r.random() == 0.9664535356921388
assert r.random() == 0.4407325991753527
state = r.getstate()
values = [r.random() for _ in range(400)]
r.setstate(state)
assert [r.random() for _ in range(400)] == values
state = r.getstate()
bits = [r.getrandbits(71) for _ in range(40)]
r.setstate(state)
assert [r.getrandbits(71) for _ in range(40)] == bits
assert math.floor(1.75) == 1
assert math.ceil(-1.75) == -1
assert math.sqrt(9) == 3
assert math.frexp(8) == (0.5, 4)
try: math.sqrt(object())
except TypeError: pass
else: raise AssertionError('invalid native math argument did not raise')
print('native scalar arguments preserve random state and math behavior')
