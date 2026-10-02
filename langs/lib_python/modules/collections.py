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

# From CPython commit 3b564385e4c9, Lib/collections/__init__.py.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# Used under the PSF license in tests/python/LICENSE.
import _collections_abc
from operator import itemgetter as _itemgetter
from heapq import nlargest as _nlargest
from itertools import chain as _chain, starmap as _starmap, repeat as _repeat

def _count_elements(mapping, iterable):
    'Tally elements from the iterable.'
    mapping_get = mapping.get
    for elem in iterable:
        mapping[elem] = mapping_get(elem, 0) + 1

try:                                    # Load C helper function if available
    from _collections import _count_elements
except ImportError:
    pass

class Counter(dict):
    '''Dict subclass for counting hashable items.  Sometimes called a bag
    or multiset.  Elements are stored as dictionary keys and their counts
    are stored as dictionary values.

    When constructed from a Mapping or Counter, the original object's
    values will be used as the initial counts.

    >>> c = Counter('abcdeabcdabcaba')  # count elements from a string

    >>> c.most_common(3)                # three most common elements
    [('a', 5), ('b', 4), ('c', 3)]
    >>> sorted(c)                       # list all unique elements
    ['a', 'b', 'c', 'd', 'e']
    >>> ''.join(sorted(c.elements()))   # list elements with repetitions
    'aaaaabbbbcccdde'
    >>> sum(c.values())                 # total of all counts
    15

    >>> c['a']                          # count of letter 'a'
    5
    >>> for elem in 'shazam':           # update counts from an iterable
    ...     c[elem] += 1                # by adding 1 to each element's count
    >>> c['a']                          # now there are seven 'a'
    7
    >>> del c['b']                      # remove all 'b'
    >>> c['b']                          # now there are zero 'b'
    0

    >>> d = Counter('simsalabim')       # make another counter
    >>> c.update(d)                     # add in the second counter
    >>> c['a']                          # now there are nine 'a'
    9

    >>> c.clear()                       # empty the counter
    >>> c
    Counter()

    Note:  If a count is set to zero or reduced to zero, it will remain
    in the counter until the entry is deleted or the counter is cleared:

    >>> c = Counter('aaabbc')
    >>> c['b'] -= 2                     # reduce the count of 'b' by two
    >>> c.most_common()                 # 'b' is still in, but its count is zero
    [('a', 3), ('c', 1), ('b', 0)]

    '''
    # References:
    #   http://en.wikipedia.org/wiki/Multiset
    #   http://www.gnu.org/software/smalltalk/manual-base/html_node/Bag.html
    #   http://www.java2s.com/Tutorial/Cpp/0380__set-multiset/Catalog0380__set-multiset.htm
    #   http://code.activestate.com/recipes/259174/
    #   Knuth, TAOCP Vol. II section 4.6.3

    def __init__(self, iterable=None, /, **kwds):
        '''Create a new, empty Counter object.  And if given, count elements
        from an input iterable.  Or, initialize the count from another mapping
        of elements to their counts.

        >>> c = Counter()                           # a new, empty counter
        >>> c = Counter('gallahad')                 # a new counter from an iterable
        >>> c = Counter({'a': 4, 'b': 2})           # a new counter from a mapping
        >>> c = Counter(a=4, b=2)                   # a new counter from keyword args

        '''
        super().__init__()
        self.update(iterable, **kwds)

    def __missing__(self, key):
        'The count of elements not in the Counter is zero.'
        # Needed so that self[missing_item] does not raise KeyError
        return 0

    def total(self):
        'Sum of the counts'
        return sum(self.values())

    def most_common(self, n=None):
        '''List the n most common elements and their counts from the most
        common to the least.  If n is None, then list all element counts.

        >>> Counter('abracadabra').most_common(3)
        [('a', 5), ('b', 2), ('r', 2)]

        '''
        # Emulate Bag.sortedByCount from Smalltalk
        if n is None:
            return sorted(self.items(), key=_itemgetter(1), reverse=True)

        return _nlargest(n, self.items(), key=_itemgetter(1))

    def elements(self):
        '''Iterator over elements repeating each as many times as its count.

        >>> c = Counter('ABCABC')
        >>> sorted(c.elements())
        ['A', 'A', 'B', 'B', 'C', 'C']

        Knuth's example for prime factors of 1836:  2**2 * 3**3 * 17**1

        >>> import math
        >>> prime_factors = Counter({2: 2, 3: 3, 17: 1})
        >>> math.prod(prime_factors.elements())
        1836

        Note, if an element's count has been set to zero or is a negative
        number, elements() will ignore it.

        '''
        # Emulate Bag.do from Smalltalk and Multiset.begin from C++.
        return _chain.from_iterable(_starmap(_repeat, self.items()))

    # Override dict methods where necessary

    @classmethod
    def fromkeys(cls, iterable, v=None):
        # There is no equivalent method for counters because the semantics
        # would be ambiguous in cases such as Counter.fromkeys('aaabbc', v=2).
        # Initializing counters to zero values isn't necessary because zero
        # is already the default value for counter lookups.  Initializing
        # to one is easily accomplished with Counter(set(iterable)).  For
        # more exotic cases, create a dictionary first using a dictionary
        # comprehension or dict.fromkeys().
        raise NotImplementedError(
            'Counter.fromkeys() is undefined.  Use Counter(iterable) instead.')

    def update(self, iterable=None, /, **kwds):
        '''Like dict.update() but add counts instead of replacing them.

        Source can be an iterable, a dictionary, or another Counter instance.

        >>> c = Counter('which')
        >>> c.update('witch')           # add elements from another iterable
        >>> d = Counter('watch')
        >>> c.update(d)                 # add elements from another counter
        >>> c['h']                      # four 'h' in which, witch, and watch
        4

        '''
        # The regular dict.update() operation makes no sense here because the
        # replace behavior results in some of the original untouched counts
        # being mixed-in with all of the other counts for a mismash that
        # doesn't have a straight-forward interpretation in most counting
        # contexts.  Instead, we implement straight-addition.  Both the inputs
        # and outputs are allowed to contain zero and negative counts.

        if iterable is not None:
            if isinstance(iterable, _collections_abc.Mapping):
                if self:
                    self_get = self.get
                    for elem, count in iterable.items():
                        self[elem] = count + self_get(elem, 0)
                else:
                    # fast path when counter is empty
                    super().update(iterable)
            else:
                _count_elements(self, iterable)
        if kwds:
            self.update(kwds)

    def subtract(self, iterable=None, /, **kwds):
        '''Like dict.update() but subtracts counts instead of replacing them.
        Counts can be reduced below zero.  Both the inputs and outputs are
        allowed to contain zero and negative counts.

        Source can be an iterable, a dictionary, or another Counter instance.

        >>> c = Counter('which')
        >>> c.subtract('witch')             # subtract elements from another iterable
        >>> c.subtract(Counter('watch'))    # subtract elements from another counter
        >>> c['h']                          # 2 in which, minus 1 in witch, minus 1 in watch
        0
        >>> c['w']                          # 1 in which, minus 1 in witch, minus 1 in watch
        -1

        '''
        if iterable is not None:
            self_get = self.get
            if isinstance(iterable, _collections_abc.Mapping):
                for elem, count in iterable.items():
                    self[elem] = self_get(elem, 0) - count
            else:
                for elem in iterable:
                    self[elem] = self_get(elem, 0) - 1
        if kwds:
            self.subtract(kwds)

    def copy(self):
        'Return a shallow copy.'
        return self.__class__(self)

    def __reduce__(self):
        return self.__class__, (dict(self),)

    def __delitem__(self, elem):
        'Like dict.__delitem__() but does not raise KeyError for missing values.'
        if elem in self:
            super().__delitem__(elem)

    def __repr__(self):
        if not self:
            return f'{self.__class__.__name__}()'
        try:
            # dict() preserves the ordering returned by most_common()
            d = dict(self.most_common())
        except TypeError:
            # handle case where values are not orderable
            d = dict(self)
        return f'{self.__class__.__name__}({d!r})'

    # Multiset-style mathematical operations discussed in:
    #       Knuth TAOCP Volume II section 4.6.3 exercise 19
    #       and at http://en.wikipedia.org/wiki/Multiset
    #
    # Outputs guaranteed to only include positive counts.
    #
    # To strip negative and zero counts, add-in an empty counter:
    #       c += Counter()
    #
    # Results are ordered according to when an element is first
    # encountered in the left operand and then by the order
    # encountered in the right operand.
    #
    # When the multiplicities are all zero or one, multiset operations
    # are guaranteed to be equivalent to the corresponding operations
    # for regular sets.
    #
    #     Given counter multisets such as:
    #         cp = Counter(a=1, b=0, c=1)
    #         cq = Counter(c=1, d=0, e=1)
    #
    #     The corresponding regular sets would be:
    #         sp = {'a', 'c'}
    #         sq = {'c', 'e'}
    #
    #     All of the following relations would hold:
    #         (cp == cq) == (sp == sq)
    #         (cp != cq) == (sp != sq)
    #         (cp <= cq) == (sp <= sq)
    #         (cp < cq) == (sp < sq)
    #         (cp >= cq) == (sp >= sq)
    #         (cp > cq) == (sp > sq)
    #         set(cp + cq) == sp | sq
    #         set(cp - cq) == sp - sq
    #         set(cp | cq) == sp | sq
    #         set(cp & cq) == sp & sq
    #         set(cp ^ cq) == sp ^ sq

    def __eq__(self, other):
        'True if all counts agree. Missing counts are treated as zero.'
        if not isinstance(other, Counter):
            return NotImplemented
        return all(self[e] == other[e] for c in (self, other) for e in c)

    def __ne__(self, other):
        'True if any counts disagree. Missing counts are treated as zero.'
        if not isinstance(other, Counter):
            return NotImplemented
        return not self == other

    def __le__(self, other):
        'True if all counts in self are a subset of those in other.'
        if not isinstance(other, Counter):
            return NotImplemented
        return all(self[e] <= other[e] for c in (self, other) for e in c)

    def __lt__(self, other):
        'True if all counts in self are a proper subset of those in other.'
        if not isinstance(other, Counter):
            return NotImplemented
        return self <= other and self != other

    def __ge__(self, other):
        'True if all counts in self are a superset of those in other.'
        if not isinstance(other, Counter):
            return NotImplemented
        return all(self[e] >= other[e] for c in (self, other) for e in c)

    def __gt__(self, other):
        'True if all counts in self are a proper superset of those in other.'
        if not isinstance(other, Counter):
            return NotImplemented
        return self >= other and self != other

    def __add__(self, other):
        '''Add counts from two counters.

        >>> Counter('abbb') + Counter('bcc')
        Counter({'b': 4, 'c': 2, 'a': 1})

        '''
        if not isinstance(other, Counter):
            return NotImplemented
        result = Counter()
        for elem, count in self.items():
            newcount = count + other[elem]
            if newcount > 0:
                result[elem] = newcount
        for elem, count in other.items():
            if elem not in self and count > 0:
                result[elem] = count
        return result

    def __sub__(self, other):
        ''' Subtract count, but keep only results with positive counts.

        >>> Counter('abbbc') - Counter('bccd')
        Counter({'b': 2, 'a': 1})

        '''
        if not isinstance(other, Counter):
            return NotImplemented
        result = Counter()
        for elem, count in self.items():
            newcount = count - other[elem]
            if newcount > 0:
                result[elem] = newcount
        for elem, count in other.items():
            if elem not in self and count < 0:
                result[elem] = 0 - count
        return result

    def __or__(self, other):
        '''Union is the maximum of value in either of the input counters.

        >>> Counter('abbb') | Counter('bcc')
        Counter({'b': 3, 'c': 2, 'a': 1})

        '''
        if not isinstance(other, Counter):
            return NotImplemented
        result = Counter()
        for elem, count in self.items():
            other_count = other[elem]
            newcount = other_count if count < other_count else count
            if newcount > 0:
                result[elem] = newcount
        for elem, count in other.items():
            if elem not in self and count > 0:
                result[elem] = count
        return result

    def __and__(self, other):
        ''' Intersection is the minimum of corresponding counts.

        >>> Counter('abbb') & Counter('bcc')
        Counter({'b': 1})

        '''
        if not isinstance(other, Counter):
            return NotImplemented
        result = Counter()
        for elem, count in self.items():
            other_count = other[elem]
            newcount = count if count < other_count else other_count
            if newcount > 0:
                result[elem] = newcount
        return result

    def __xor__(self, other):
        '''Symmetric difference. Absolute value of count differences.

        The symmetric difference p ^ q is equivalent to:

            (p - q) | (q - p).

        For each element, symmetric difference gives the same result as:

            max(p[elem], q[elem]) - min(p[elem], q[elem])

        >>> Counter(a=5, b=3, c=2, d=2) ^ Counter(a=1, b=3, c=5, e=1)
        Counter({'a': 4, 'c': 3, 'd': 2, 'e': 1})

        '''
        if not isinstance(other, Counter):
            return NotImplemented
        result = Counter()
        for elem, count in self.items():
            newcount = abs(count - other[elem])
            if newcount:
                result[elem] = newcount
        for elem, count in other.items():
            if elem not in self and count:
                result[elem] = abs(count)
        return result

    def __pos__(self):
        'Adds an empty counter, effectively stripping negative and zero counts'
        result = Counter()
        for elem, count in self.items():
            if count > 0:
                result[elem] = count
        return result

    def __neg__(self):
        '''Subtracts from an empty counter.  Strips positive and zero counts,
        and flips the sign on negative counts.

        '''
        result = Counter()
        for elem, count in self.items():
            if count < 0:
                result[elem] = 0 - count
        return result

    def _keep_positive(self):
        '''Internal method to strip elements with a negative or zero count'''
        nonpositive = [elem for elem, count in self.items() if not count > 0]
        for elem in nonpositive:
            del self[elem]
        return self

    def __iadd__(self, other):
        '''Inplace add from another counter, keeping only positive counts.

        >>> c = Counter('abbb')
        >>> c += Counter('bcc')
        >>> c
        Counter({'b': 4, 'c': 2, 'a': 1})

        '''
        for elem, count in other.items():
            self[elem] += count
        return self._keep_positive()

    def __isub__(self, other):
        '''Inplace subtract counter, but keep only results with positive counts.

        >>> c = Counter('abbbc')
        >>> c -= Counter('bccd')
        >>> c
        Counter({'b': 2, 'a': 1})

        '''
        for elem, count in other.items():
            self[elem] -= count
        return self._keep_positive()

    def __ior__(self, other):
        '''Inplace union is the maximum of value from either counter.

        >>> c = Counter('abbb')
        >>> c |= Counter('bcc')
        >>> c
        Counter({'b': 3, 'c': 2, 'a': 1})

        '''
        for elem, other_count in other.items():
            count = self[elem]
            if other_count > count:
                self[elem] = other_count
        return self._keep_positive()

    def __iand__(self, other):
        '''Inplace intersection is the minimum of corresponding counts.

        >>> c = Counter('abbb')
        >>> c &= Counter('bcc')
        >>> c
        Counter({'b': 1})

        '''
        for elem, count in self.items():
            other_count = other[elem]
            if other_count < count:
                self[elem] = other_count
        return self._keep_positive()

    def __ixor__(self, other):
        '''Inplace symmetric difference. Absolute value of count differences.

        >>> c = Counter(a=5, b=3, c=2, d=2)
        >>> c ^= Counter(a=1, b=3, c=5, e=1)
        >>> c
        Counter({'a': 4, 'c': 3, 'd': 2, 'e': 1})

        '''
        for elem, count in self.items():
            self[elem] = abs(count - other[elem])
        for elem, count in other.items():
            if elem not in self:
                self[elem] = abs(count)
        return self._keep_positive()




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
