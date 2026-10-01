# Python implementations of the C-only collections containers.
# Behaviour follows CPython 3b564385e4c9, Modules/_collectionsmodule.c (PSF License).
from operator import index as _index
from reprlib import recursive_repr as _recursive_repr
import sys as _sys


def _deque_size(value):
    number = _index(value)
    if number > _sys.maxsize or number < -_sys.maxsize - 1:
        raise OverflowError('Python int too large to convert to C ssize_t')
    return number


def _deque_field(owner, name):
    return deque.__dict__[name].__get__(owner, deque)


def _deque_store(owner, name, value):
    deque.__dict__[name].__set__(owner, value)


def _deque_push(owner, value, left=False):
    if left:
        _deque_field(owner, '_data').insert(0, value)
    else:
        _deque_field(owner, '_data').append(value)
    limit = _deque_field(owner, '_maxlen')
    if limit is not None and len(_deque_field(owner, '_data')) > limit:
        _deque_field(owner, '_data').pop(-1 if left else 0)
    _deque_store(owner, '_state', _deque_field(owner, '_state') + 1)


def _deque_method(name, minimum, maximum):
    def decorate(function):
        def invoke(*given, **keywords):
            if not given:
                raise TypeError('unbound method deque.' + name + '() needs an argument')
            self, args = given[0], given[1:]
            if not isinstance(self, deque):
                raise TypeError("descriptor '" + name + "' for 'collections.deque' objects doesn't apply to a '" + type(self).__name__ + "' object")
            if keywords:
                raise TypeError('deque.' + name + '() takes no keyword arguments')
            given = len(args)
            if given < minimum or given > maximum:
                if minimum == maximum == 1:
                    message = 'deque.' + name + '() takes exactly one argument (' + str(given) + ' given)'
                elif maximum == 0:
                    message = 'deque.' + name + '() takes no arguments (' + str(given) + ' given)'
                elif name == 'rotate':
                    message = 'deque.rotate expected at most 1 argument, got ' + str(given)
                elif minimum == maximum:
                    message = name + '() takes exactly ' + str(maximum) + ' arguments (' + str(given) + ' given)'
                elif given < minimum:
                    message = name + '() takes at least ' + str(minimum) + ' argument' + ('s' if minimum != 1 else '') + ' (' + str(given) + ' given)'
                else:
                    message = name + '() takes at most ' + str(maximum) + ' argument' + ('s' if maximum != 1 else '') + ' (' + str(given) + ' given)'
                raise TypeError(message)
            return function(self, *args)
        invoke.__name__ = name
        invoke.__qualname__ = 'deque.' + name
        invoke.__doc__ = function.__doc__
        return invoke
    return decorate


def _deque_protocol(name, count):
    def decorate(function):
        def invoke(*given, **keywords):
            if not given:
                raise TypeError("descriptor '" + name + "' of 'collections.deque' object needs an argument")
            self, args = given[0], given[1:]
            if not isinstance(self, deque):
                raise TypeError("descriptor '" + name + "' requires a 'collections.deque' object but received a '" + type(self).__name__ + "'")
            if keywords:
                raise TypeError('wrapper ' + name + '() takes no keyword arguments')
            if len(args) != count:
                prefix = ' ' if name in ('__setitem__', '__mul__', '__rmul__', '__imul__') else ''
                raise TypeError(prefix + 'expected ' + str(count) + ' argument' + ('s' if count != 1 else '') + ', got ' + str(len(args)))
            return function(self, *args)
        invoke.__name__ = name
        invoke.__qualname__ = 'deque.' + name
        invoke.__doc__ = function.__doc__
        return invoke
    return decorate


def _deque_iter_args(args):
    count = len(args)
    if count < 1:
        raise TypeError('function takes at least 1 argument (0 given)')
    if count > 2:
        raise TypeError('function takes at most 2 arguments (' + str(count) + ' given)')
    return args[0], args[1] if count == 2 else 0


def _deque_args(args, keywords):
    count = len(args) + len(keywords)
    if count > 2:
        raise TypeError('deque() takes at most 2 arguments (' + str(count) + ' given)')
    values = [(), None]
    names = ('iterable', 'maxlen')
    for key in keywords:
        if key not in names:
            raise TypeError("'" + key + "' is an invalid keyword argument for deque()")
    for position, value in enumerate(args):
        if names[position] in keywords:
            raise TypeError("argument for deque() given by name ('" + names[position] + "') and position (" + str(position + 1) + ")")
        values[position] = value
    for position, name in enumerate(names):
        if name in keywords:
            values[position] = keywords[name]
    return values


def _deque_iter_setup(iterator, owner, index, reverse):
    if not isinstance(owner, deque):
        raise TypeError('argument 1 must be collections.deque, not ' + type(owner).__name__)
    index = _deque_size(index)
    iterator._owner = owner
    iterator._state = _deque_field(owner, '_state')
    iterator._position = len(_deque_field(owner, '_data')) - 1 if reverse else 0
    iterator._remaining = len(_deque_field(owner, '_data'))
    iterator._reverse = reverse
    for _ in range(min(max(0, index), iterator._remaining)):
        next(iterator)


class _deque_iterator:
    __slots__ = ('_owner', '_state', '_position', '_remaining', '_reverse')
    __native_type_name__ = '_collections._deque_iterator'

    def __init__(self, *args, **kwargs):
        owner, index = _deque_iter_args(args)
        _deque_iter_setup(self, owner, index, False)

    def __iter__(self):
        return self

    def __next__(self):
        owner = self._owner
        if self._reverse and not self._remaining:
            raise StopIteration
        if _deque_field(owner, '_state') != self._state:
            self._remaining = 0
            raise RuntimeError('deque mutated during iteration')
        if not self._remaining:
            raise StopIteration
        result = _deque_field(owner, '_data')[self._position]
        self._position += -1 if self._reverse else 1
        self._remaining -= 1
        return result

    def __length_hint__(self):
        return self._remaining

    def __reduce__(self):
        return (type(self), (self._owner, len(self.__deque_field(owner, '_data')) - self._remaining))


class _deque_reverse_iterator:
    __slots__ = ('_owner', '_state', '_position', '_remaining', '_reverse')
    __native_type_name__ = '_collections._deque_reverse_iterator'

    def __init__(self, *args, **kwargs):
        owner, index = _deque_iter_args(args)
        _deque_iter_setup(self, owner, index, True)

    __iter__ = _deque_iterator.__iter__
    __next__ = _deque_iterator.__next__
    __length_hint__ = _deque_iterator.__length_hint__
    __reduce__ = _deque_iterator.__reduce__


class deque:
    """A list-like sequence optimized for data accesses near its endpoints."""
    __slots__ = ('_data', '_maxlen', '_state', '__weakref__')
    __hash__ = None
    __module__ = 'collections'
    __native_type_name__ = 'collections.deque'
    __abc_tpflags__ = 32

    def __new__(*given, **kwargs):
        if not given:
            raise TypeError('collections.deque.__new__(): not enough arguments')
        cls = given[0]
        if not isinstance(cls, type):
            raise TypeError('collections.deque.__new__(X): X is not a type object (' + type(cls).__name__ + ')')
        if not issubclass(cls, deque):
            raise TypeError('collections.deque.__new__(' + cls.__name__ + '): ' + cls.__name__ + ' is not a subtype of collections.deque')
        owner = object.__new__(cls)
        _deque_store(owner, '_data', [])
        _deque_store(owner, '_maxlen', None)
        _deque_store(owner, '_state', 0)
        return owner

    def __init__(*given, **kwargs):
        if not given:
            raise TypeError("descriptor '__init__' of 'collections.deque' object needs an argument")
        self, args = given[0], given[1:]
        if not isinstance(self, deque):
            raise TypeError("descriptor '__init__' requires a 'collections.deque' object but received a '" + type(self).__name__ + "'")
        iterable, maxlen = _deque_args(args, kwargs)
        if maxlen is not None and not isinstance(maxlen, int):
            raise TypeError('an integer is required')
        limit = None if maxlen is None else _deque_size(maxlen)
        if limit is not None and limit < 0:
            raise ValueError('maxlen must be non-negative')
        _deque_store(self, '_maxlen', limit)
        deque.clear(self)
        deque.extend(self, iterable)

    @property
    def maxlen(self):
        return _deque_field(self, '_maxlen')

    @maxlen.setter
    def maxlen(self, value):
        raise AttributeError("attribute 'maxlen' of 'collections.deque' objects is not writable")

    @maxlen.deleter
    def maxlen(self):
        raise AttributeError("attribute 'maxlen' of 'collections.deque' objects is not writable")

    @_deque_method('append', 1, 1)
    def append(self, value, /):
        _deque_push(self, value)

    @_deque_method('appendleft', 1, 1)
    def appendleft(self, value, /):
        _deque_push(self, value, True)

    @_deque_method('pop', 0, 0)
    def pop(self):
        if not _deque_field(self, '_data'):
            raise IndexError('pop from an empty deque')
        _deque_store(self, '_state', _deque_field(self, '_state') + 1)
        return _deque_field(self, '_data').pop()

    @_deque_method('popleft', 0, 0)
    def popleft(self):
        if not _deque_field(self, '_data'):
            raise IndexError('pop from an empty deque')
        _deque_store(self, '_state', _deque_field(self, '_state') + 1)
        return _deque_field(self, '_data').pop(0)

    @_deque_method('extend', 1, 1)
    def extend(self, iterable, /):
        if iterable is self:
            iterable = list(_deque_field(self, '_data'))
        for value in iterable:
            if _deque_field(self, '_maxlen') != 0:
                _deque_push(self, value)

    @_deque_method('extendleft', 1, 1)
    def extendleft(self, iterable, /):
        if iterable is self:
            iterable = list(_deque_field(self, '_data'))
        for value in iterable:
            if _deque_field(self, '_maxlen') != 0:
                _deque_push(self, value, True)

    @_deque_method('clear', 0, 0)
    def clear(self):
        if _deque_field(self, '_data'):
            _deque_store(self, '_data', [])
            _deque_store(self, '_state', _deque_field(self, '_state') + 1)

    @_deque_method('rotate', 0, 1)
    def rotate(self, n=1, /):
        n = _deque_size(n)
        size = len(_deque_field(self, '_data'))
        if size > 1:
            _deque_store(self, '_state', _deque_field(self, '_state') + 1)
            n %= size
            _deque_store(self, '_data', _deque_field(self, '_data')[-n:] + _deque_field(self, '_data')[:-n] if n else _deque_field(self, '_data')[:])

    @_deque_method('reverse', 0, 0)
    def reverse(self):
        _deque_field(self, '_data').reverse()

    @_deque_method('count', 1, 1)
    def count(self, value, /):
        result = 0
        for item in _deque_iterator(self):
            if item is value or item == value:
                result += 1
        return result

    @_deque_method('index', 1, 3)
    def index(self, value, start=0, stop=_sys.maxsize, /):
        start, stop = _index(start), _index(stop)
        start = min(_sys.maxsize, max(-_sys.maxsize - 1, start))
        stop = min(_sys.maxsize, max(-_sys.maxsize - 1, stop))
        size = len(_deque_field(self, '_data'))
        if start < 0:
            start = max(0, size + start)
        if stop < 0:
            stop = max(0, size + stop)
        stop = min(stop, size)
        state = _deque_field(self, '_state')
        for position in range(start, stop):
            item = _deque_field(self, '_data')[position]
            found = item is value or item == value
            if found:
                return position
            if state != _deque_field(self, '_state'):
                raise RuntimeError('deque mutated during iteration')
        raise ValueError('deque.index(x): x not in deque')

    @_deque_method('insert', 2, 2)
    def insert(self, position, value, /):
        position = _deque_size(position)
        if _deque_field(self, '_maxlen') is not None and len(_deque_field(self, '_data')) == _deque_field(self, '_maxlen'):
            raise IndexError('deque already at its maximum size')
        _deque_field(self, '_data').insert(position, value)
        _deque_store(self, '_state', _deque_field(self, '_state') + 1)

    @_deque_method('remove', 1, 1)
    def remove(self, value, /):
        state = _deque_field(self, '_state')
        for position in range(len(_deque_field(self, '_data'))):
            item = _deque_field(self, '_data')[position]
            found = item is value or item == value
            if state != _deque_field(self, '_state'):
                raise IndexError('deque mutated during iteration')
            if found:
                _deque_field(self, '_data').pop(position)
                _deque_store(self, '_state', _deque_field(self, '_state') + 1)
                return
        raise ValueError('deque.remove(x): x not in deque')

    @_deque_method('copy', 0, 0)
    def copy(self):
        cls = type(self)
        result = cls(self) if _deque_field(self, '_maxlen') is None else cls(self, _deque_field(self, '_maxlen'))
        if not isinstance(result, deque):
            raise TypeError(cls.__name__ + '() must return a deque, not ' + type(result).__name__)
        return result

    @_deque_method('__copy__', 0, 0)
    def __copy__(self):
        return deque.copy(self)

    @_deque_protocol('__len__', 0)
    def __len__(self):
        return len(_deque_field(self, '_data'))

    @_deque_protocol('__iter__', 0)
    def __iter__(self):
        return _deque_iterator(self)

    @_deque_method('__reversed__', 0, 0)
    def __reversed__(self):
        return _deque_reverse_iterator(self)

    @_deque_protocol('__getitem__', 1)
    def __getitem__(self, position):
        if not isinstance(position, int) and not hasattr(type(position), '__index__'):
            raise TypeError("sequence index must be integer, not '" + type(position).__name__ + "'")
        position = _index(position)
        if position > _sys.maxsize or position < -_sys.maxsize - 1:
            raise IndexError("cannot fit 'int' into an index-sized integer")
        size = len(_deque_field(self, '_data'))
        if not -size <= position < size:
            raise IndexError('deque index out of range')
        return _deque_field(self, '_data')[position]

    @_deque_protocol('__setitem__', 2)
    def __setitem__(self, position, value):
        position = _index(position)
        if position > _sys.maxsize or position < -_sys.maxsize - 1:
            raise IndexError("cannot fit 'int' into an index-sized integer")
        size = len(_deque_field(self, '_data'))
        if not -size <= position < size:
            raise IndexError('deque index out of range')
        data = list(_deque_field(self, '_data'))
        data[position] = value
        _deque_store(self, '_data', data)

    @_deque_protocol('__delitem__', 1)
    def __delitem__(self, position):
        position = _index(position)
        if position > _sys.maxsize or position < -_sys.maxsize - 1:
            raise IndexError("cannot fit 'int' into an index-sized integer")
        size = len(_deque_field(self, '_data'))
        if not -size <= position < size:
            raise IndexError('deque index out of range')
        _deque_field(self, '_data').pop(position)
        _deque_store(self, '_state', _deque_field(self, '_state') + 1)

    @_deque_protocol('__contains__', 1)
    def __contains__(self, value):
        state = _deque_field(self, '_state')
        for item in _deque_iterator(self):
            found = item is value or item == value
            if found:
                return True
            if state != _deque_field(self, '_state'):
                raise RuntimeError('deque mutated during iteration')
        return False

    @_deque_protocol('__eq__', 1)
    def __eq__(self, other):
        if not isinstance(other, deque):
            return NotImplemented
        if self is other:
            return True
        if len(_deque_field(self, '_data')) != len(_deque_field(other, '_data')):
            return False
        return all(a is b or a == b for a, b in zip(iter(self), iter(other)))

    @_deque_protocol('__ne__', 1)
    def __ne__(self, other):
        equal = deque.__eq__(self, other)
        return equal if equal is NotImplemented else not equal

    def _compare(self, other, operation):
        if not isinstance(other, deque):
            return NotImplemented
        for a, b in zip(iter(self), iter(other)):
            if a is b or a == b:
                continue
            return operation(a, b)
        return operation(len(_deque_field(self, '_data')), len(_deque_field(other, '_data')))

    @_deque_protocol('__lt__', 1)
    def __lt__(self, other):
        from operator import lt
        return deque._compare(self, other, lt)

    @_deque_protocol('__le__', 1)
    def __le__(self, other):
        from operator import le
        return deque._compare(self, other, le)

    @_deque_protocol('__gt__', 1)
    def __gt__(self, other):
        from operator import gt
        return deque._compare(self, other, gt)

    @_deque_protocol('__ge__', 1)
    def __ge__(self, other):
        from operator import ge
        return deque._compare(self, other, ge)

    @_deque_protocol('__add__', 1)
    def __add__(self, other):
        if not isinstance(other, deque):
            raise TypeError('can only concatenate deque (not "' + type(other).__name__ + '") to deque')
        result = deque.copy(self)
        deque.extend(result, other)
        return result

    @_deque_protocol('__iadd__', 1)
    def __iadd__(self, other):
        deque.extend(self, other)
        return self

    @_deque_protocol('__mul__', 1)
    def __mul__(self, times):
        result = deque.copy(self)
        deque.__imul__(result, times)
        return result

    @_deque_protocol('__rmul__', 1)
    def __rmul__(self, other):
        return deque.__mul__(self, other)

    @_deque_protocol('__imul__', 1)
    def __imul__(self, times):
        times = _deque_size(times)
        if not _deque_field(self, '_data') or times == 1:
            return self
        if times <= 0:
            deque.clear(self)
            return self
        size = len(_deque_field(self, '_data'))
        if size > _sys.maxsize // times:
            raise MemoryError
        old = _deque_field(self, '_data')[:]
        if _deque_field(self, '_maxlen') is not None:
            times = min(times, (_deque_field(self, '_maxlen') + size - 1) // size + 1)
        for _ in range(times - 1):
            deque.extend(self, old)
        return self

    @_deque_protocol('__repr__', 0)
    @_recursive_repr('[...]')
    def __repr__(self):
        result = type(self).__name__ + '(' + repr(list(self))
        if _deque_field(self, '_maxlen') is not None:
            result += ', maxlen=' + str(_deque_field(self, '_maxlen'))
        return result + ')'

    @_deque_method('__reduce__', 0, 0)
    def __reduce__(self):
        args = () if _deque_field(self, '_maxlen') is None else ((), _deque_field(self, '_maxlen'))
        state = getattr(self, '__dict__', None)
        return (type(self), args, state, iter(self))

    def __class_getitem__(cls, item):
        from types import GenericAlias
        return GenericAlias(cls, item)


class defaultdict(dict):
    def __new__(cls, default_factory=None, *args, **kwargs):
        return super().__new__(cls)

    def __init__(self, default_factory=None, *args, **kwargs):
        if default_factory is not None and not callable(default_factory):
            raise TypeError('first argument must be callable or None')
        self.default_factory = default_factory
        super().__init__(*args, **kwargs)

    def __getitem__(self, key):
        return dict.__getitem__(self, key)

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


# The C-only field descriptor used by the pure Python namedtuple factory.
class _tuplegetter:
    def __init__(self, index, doc):
        from operator import index as as_index
        self._index = as_index(index)
        self.__doc__ = doc

    def __get__(self, instance, owner=None):
        if instance is None:
            return self
        if not isinstance(instance, tuple):
            raise TypeError("descriptor for index '%s' for tuple subclasses doesn't apply to a '%s' object" % (self._index, type(instance).__name__))
        return instance[self._index]

    def __set__(self, instance, value):
        raise AttributeError('readonly attribute')

    def __delete__(self, instance):
        raise AttributeError('readonly attribute')

__seal_class(_deque_iterator)
__seal_class(_deque_reverse_iterator)
