# These routines ask the ordinary operators to do their own work.
def add(a, b):
    return a + b

def sub(a, b):
    return a - b

def mul(a, b):
    return a * b

def truediv(a, b):
    return a / b

def floordiv(a, b):
    return a // b

def mod(a, b):
    return a % b

def pow(a, b):
    return a ** b

def neg(a):
    return -a

def pos(a):
    return +a

def abs(a):
    if a < 0:
        return -a
    return a

def eq(a, b):
    return a == b

def ne(a, b):
    return a != b

def lt(a, b):
    return a < b

def le(a, b):
    return a <= b

def gt(a, b):
    return a > b

def ge(a, b):
    return a >= b

def is_(a, b):
    return a is b

def is_not(a, b):
    return a is not b

def not_(a):
    return not a

def truth(a):
    return not not a

def contains(a, b):
    return b in a

def getitem(a, b):
    return a[b]

def index(a):
    if type(a) != type(1) and type(a) != type(True):
        raise 'TypeError: value cannot be interpreted as an integer'
    return int(a)

class _ItemGetter:
    def __init__(self, names):
        self.names = names

    def take(self, value):
        if len(self.names) == 1:
            return value[self.names[0]]
        return [value[name] for name in self.names]

def itemgetter(*items):
    if len(items) == 0:
        raise 'TypeError: itemgetter needs at least one item'
    return _ItemGetter(items).take

class _AttrGetter:
    def __init__(self, names):
        self.names = names

    def take(self, value):
        result = []
        for name in self.names:
            # A dotted attribute is followed one word at a time.
            word = ''
            held = value
            for ch in list(name):
                if ch == '.':
                    held = getattr(held, word)
                    word = ''
                else:
                    word += ch
            result.append(getattr(held, word))
        if len(result) == 1:
            return result[0]
        return result

def attrgetter(*names):
    if len(names) == 0:
        raise 'TypeError: attrgetter needs at least one attribute'
    for name in names:
        if type(name) != type(''):
            raise 'TypeError: attribute names must be strings'
    return _AttrGetter(names).take

# Bit work can be written with division and remainders, including the
# sign extension of negative whole numbers.
def _bits(a, b, operation):
    a = index(a)
    b = index(b)
    result = 0
    place = 1
    while a not in (0, -1) or b not in (0, -1):
        left = a - (a // 2) * 2
        right = b - (b // 2) * 2
        if operation == 'and':
            digit = left * right
        elif operation == 'or':
            digit = 1 if left + right != 0 else 0
        else:
            digit = (left + right) % 2
        result += digit * place
        place *= 2
        a //= 2
        b //= 2
    negative = (a == -1 and b == -1) if operation == 'and' else (a == -1 or b == -1)
    if operation == 'xor':
        negative = a != b
    if negative:
        result -= place
    return result

def and_(a, b):
    return _bits(a, b, 'and')

def or_(a, b):
    return _bits(a, b, 'or')

def xor(a, b):
    return _bits(a, b, 'xor')

def invert(a):
    return -index(a) - 1

def lshift(a, b):
    a, b = index(a), index(b)
    if b < 0:
        raise 'ValueError: negative shift count'
    return a * 2 ** b

def rshift(a, b):
    a, b = index(a), index(b)
    if b < 0:
        raise 'ValueError: negative shift count'
    return a // 2 ** b

def concat(a, b):
    if type(a) == type([]) and type(b) == type([]):
        return [*a, *b]
    return a + b

def matmul(a, b):
    raise 'NotImplementedError: matrix multiplication is not supported'

def countOf(sequence, value):
    return sum([1 for item in sequence if item == value])

def indexOf(sequence, value):
    position = 0
    for item in sequence:
        if item == value:
            return position
        position += 1
    raise 'ValueError: sequence.index(x): x not in sequence'

def length_hint(value, default=0):
    if type(value) == type([]) or type(value) == type({}) or type(value) == type(''):
        return len(value)
    # A user iterator's length hint cannot yet be consulted.
    raise 'NotImplementedError: length hints need iterator methods'

def setitem(sequence, key, value):
    raise 'NotImplementedError: setitem needs shared mutable sequence storage'
