# The small containers below keep their contents in ordinary arrays and
# maps. Where the object protocol is wanting, no silent stand-in is used.
class OrderedDict(dict):
    def move_to_end(self, key, last=True):
        value = self.pop(key)
        if last:
            self[key] = value
        else:
            remaining = list(self.items())
            self.clear()
            self[key] = value
            self.update(remaining)


def namedtuple(typename, field_names, *, rename=False, defaults=None, module=None):
    from _namedtuple import namedtuple as make_record
    return make_record(typename, field_names, rename=rename, defaults=defaults, module=module)

# Named records bear fields of their own. Tuple indexing, immutability
# and the remaining tuple methods await the object protocol.
class _Record:
    def __init__(self, *values):
        if len(values) != len(self._fields):
            raise 'TypeError: wrong number of namedtuple fields'
        for i in range(len(values)):
            setattr(self, self._fields[i], values[i])

class deque:
    def __init__(self, iterable=None, maxlen=None):
        self.data = [] if iterable is None else list(iterable)
        self.maxlen = maxlen
        if maxlen is not None:
            if maxlen < 0:
                raise 'ValueError: maxlen must be non-negative'
            self.data = self.data[len(self.data) - maxlen:] if maxlen else []

    def append(self, value):
        self.data = [*self.data, value]
        if self.maxlen is not None and len(self.data) > self.maxlen:
            self.data = self.data[1:]

    def appendleft(self, value):
        self.data = [value, *self.data]
        if self.maxlen is not None and len(self.data) > self.maxlen:
            self.data = self.data[:-1]

    def pop(self):
        if len(self.data) == 0:
            raise 'IndexError: pop from an empty deque'
        value = self.data[len(self.data) - 1]
        self.data = self.data[:-1]
        return value

    def popleft(self):
        if len(self.data) == 0:
            raise 'IndexError: pop from an empty deque'
        value = self.data[0]
        self.data = self.data[1:]
        return value

    def extend(self, values):
        for value in values:
            self.append(value)

    def extendleft(self, values):
        for value in values:
            self.appendleft(value)

    def clear(self):
        self.data = []

    def rotate(self, n=1):
        if len(self.data) != 0:
            n %= len(self.data)
            self.data = [*self.data[-n:], *self.data[:-n]]

    def count(self, value):
        return sum([1 for held in self.data if held == value])

    def __len__(self):
        return len(self.data)

    def __iter__(self):
        return iter(self.data)

    def __reversed__(self):
        return reversed(self.data)

    def __getitem__(self, index):
        return self.data[index]

    def __contains__(self, value):
        for held in self.data:
            if held is value or held == value:
                return True
        return False

    def __eq__(self, other):
        if isinstance(other, deque):
            return self.data == other.data
        return NotImplemented

    def __repr__(self):
        return 'deque(' + repr(self.data) + ')'

class defaultdict(dict):
    def __new__(cls, default_factory=None, *args, **kwargs):
        return super().__new__(cls)

    def __init__(self, default_factory=None, *args, **kwargs):
        if default_factory is not None and not callable(default_factory):
            raise TypeError('first argument must be callable or None')
        self.default_factory = default_factory
        super().__init__(*args, **kwargs)

    def __getitem__(self, key):
        try:
            return dict.__getitem__(self, key)
        except KeyError:
            return self.__missing__(key)

    def __missing__(self, key):
        if self.default_factory is None:
            raise KeyError(key)
        self[key] = value = self.default_factory()
        return value

    def __repr__(self):
        return 'defaultdict(' + repr(self.default_factory) + ', ' + dict.__repr__(self) + ')'

    def copy(self):
        return type(self)(self.default_factory, self)

    def __copy__(self):
        return self.copy()

    def __reduce__(self):
        args = (self.default_factory,)
        return type(self), args, None, None, iter(self.items())

class Counter:
    def __init__(self, iterable=None, **kwargs):
        raise 'NotImplementedError: Counter needs object indexing methods'


# A list and a dictionary written out in Python, for a program that
# wants to inherit from one and change a part of it. Each keeps its
# contents in an ordinary list or map under the name data, as CPython's
# do, and takes the rest of its behaviour from the kind it stands on.
from collections.abc import MutableSequence, MutableMapping


def _is_slice(index):
    # The reader hands a slice to an object's own __getitem__, but
    # isinstance cannot be asked about the slice kind, hasattr says no
    # for its parts, and reaching for one on something else fails in a
    # way no try can hold. What is left is how a slice writes itself
    # out. A thing of a program's own whose repr begins the same way is
    # taken for one, which is the price of asking the question this way.
    return repr(index)[:6] == 'slice('


class UserList(MutableSequence):
    def __init__(self, initlist=None):
        self.data = []
        if initlist is not None:
            if isinstance(initlist, UserList):
                self.data = list(initlist.data)
            else:
                self.data = list(initlist)

    def __repr__(self):
        return repr(self.data)

    def __len__(self):
        return len(self.data)

    def __getitem__(self, index):
        if _is_slice(index):
            return type(self)(self.data[index])
        return self.data[index]

    def __setitem__(self, index, value):
        self.data[index] = value

    def __delitem__(self, index):
        del self.data[index]

    def __contains__(self, value):
        return value in self.data

    def __iter__(self):
        return iter(self.data)

    def __eq__(self, other):
        return self.data == self.__cast(other)

    def __ne__(self, other):
        return not self.__eq__(other)

    def __lt__(self, other):
        return self.data < self.__cast(other)

    def __le__(self, other):
        return self.data <= self.__cast(other)

    def __gt__(self, other):
        return self.data > self.__cast(other)

    def __ge__(self, other):
        return self.data >= self.__cast(other)

    def __cast(self, other):
        if isinstance(other, UserList):
            return other.data
        return other

    def __add__(self, other):
        return type(self)(self.data + list(self.__cast(other)))

    def __radd__(self, other):
        return type(self)(list(self.__cast(other)) + self.data)

    def __iadd__(self, other):
        self.data += list(self.__cast(other))
        return self

    def __mul__(self, count):
        return type(self)(self.data * count)

    def __rmul__(self, count):
        return self.__mul__(count)

    def __imul__(self, count):
        self.data *= count
        return self

    def append(self, value):
        self.data.append(value)

    def insert(self, index, value):
        self.data.insert(index, value)

    def pop(self, index=-1):
        return self.data.pop(index)

    def remove(self, value):
        self.data.remove(value)

    def clear(self):
        self.data.clear()

    def copy(self):
        return type(self)(self)

    def count(self, value):
        return self.data.count(value)

    def index(self, value, *rest):
        return self.data.index(value, *rest)

    def reverse(self):
        self.data.reverse()

    def sort(self, *args, **keywords):
        self.data.sort(*args, **keywords)

    def extend(self, other):
        if isinstance(other, UserList):
            self.data.extend(other.data)
        else:
            self.data.extend(other)


class UserDict(MutableMapping):
    def __init__(self, initial=None, **keywords):
        self.data = {}
        if initial is not None:
            self.update(initial)
        if keywords:
            self.update(keywords)

    def __repr__(self):
        return repr(self.data)

    def __len__(self):
        return len(self.data)

    def __getitem__(self, key):
        if key in self.data:
            return self.data[key]
        if hasattr(type(self), '__missing__'):
            return type(self).__missing__(self, key)
        raise KeyError(key)

    def __setitem__(self, key, value):
        self.data[key] = value

    def __delitem__(self, key):
        del self.data[key]

    def __iter__(self):
        return iter(self.data)

    def __contains__(self, key):
        return key in self.data

    def __eq__(self, other):
        if isinstance(other, UserDict):
            return self.data == other.data
        return self.data == other

    def __ne__(self, other):
        return not self.__eq__(other)

    def copy(self):
        return type(self)(self.data)

    def keys(self):
        return self.data.keys()

    def values(self):
        return self.data.values()

    def items(self):
        return self.data.items()

    def get(self, key, default=None):
        if key in self.data:
            return self.data[key]
        return default

    @classmethod
    def fromkeys(cls, iterable, value=None):
        made = cls()
        for key in iterable:
            made[key] = value
        return made
