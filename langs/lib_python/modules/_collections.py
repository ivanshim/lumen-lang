# Python implementations of the C-only collections containers.
# Behaviour follows CPython 3b564385e4c9, Modules/_collectionsmodule.c (PSF License).
from operator import index as _index
from operator import eq as _eq, ne as _ne, lt as _lt, le as _le, gt as _gt, ge as _ge
from reprlib import recursive_repr as _recursive_repr
import sys as _sys


def _deque_size(value):
    number = _index(value)
    if number > _sys.maxsize or number < -_sys.maxsize - 1:
        raise OverflowError('Python int too large to convert to C ssize_t')
    return number


# A deque keeps one slot, a list [items, maxlen, state, careful], read and
# written through the descriptor the class itself owns, so that a subclass
# (or its __getattribute__ or __setattr__) can never intercept it.  It is
# bound once, below the class.  The state counts the mutations that change
# the items' length or order.  The end operations skip that count, and the
# maxlen test, until the deque is careful: it has a maxlen, or an iterator
# or a scan has looked at it, since nobody can have noticed the difference
# before.
def _deque_bind(name):
    return deque.__dict__[name].__get__, deque.__dict__[name].__set__


# Marks a missing argument of the methods that take their arguments by name
# so as to skip packing them (see deque.append).
_MISSING = object()
_object_new = object.__new__


def _given(extra, *head):
    # The positional arguments a method was called with, for its error messages.
    if head[0] is _MISSING:
        return ()
    return tuple([item for item in head if item is not _MISSING]) + extra


def _deque_check(name, minimum, maximum, given, keywords):
    # The argument checks of a deque method called as deque.<name>(*given, **keywords).
    if not given:
        raise TypeError('unbound method deque.' + name + '() needs an argument')
    self, count = given[0], len(given) - 1
    if not isinstance(self, deque):
        raise TypeError("descriptor '" + name + "' for 'collections.deque' objects doesn't apply to a '" + type(self).__name__ + "' object")
    if keywords:
        raise TypeError('deque.' + name + '() takes no keyword arguments')
    if count < minimum or count > maximum:
        if minimum == maximum == 1:
            message = 'deque.' + name + '() takes exactly one argument (' + str(count) + ' given)'
        elif maximum == 0:
            message = 'deque.' + name + '() takes no arguments (' + str(count) + ' given)'
        elif minimum == maximum:
            message = name + ' expected ' + str(maximum) + ' arguments, got ' + str(count)
        elif count < minimum:
            message = name + ' expected at least ' + str(minimum) + ' argument' + ('s' if minimum != 1 else '') + ', got ' + str(count)
        else:
            message = name + ' expected at most ' + str(maximum) + ' argument' + ('s' if maximum != 1 else '') + ', got ' + str(count)
        raise TypeError(message)


def _deque_method(name, minimum, maximum):
    def decorate(function):
        def invoke(*given, **keywords):
            _deque_check(name, minimum, maximum, given, keywords)
            return function(*given)
        invoke.__name__ = name
        invoke.__qualname__ = 'deque.' + name
        invoke.__doc__ = function.__doc__
        return invoke
    return decorate


def _deque_slot_check(name, count, given, keywords):
    # The argument checks of a deque slot wrapper such as deque.__len__(*given).
    if not given:
        raise TypeError("descriptor '" + name + "' of 'collections.deque' object needs an argument")
    self = given[0]
    if not isinstance(self, deque):
        raise TypeError("descriptor '" + name + "' requires a 'collections.deque' object but received a '" + type(self).__name__ + "'")
    if keywords:
        raise TypeError('wrapper ' + name + '() takes no keyword arguments')
    if len(given) - 1 != count:
        prefix = name + ' ' if name == '__setitem__' else ''
        raise TypeError(prefix + 'expected ' + str(count) + ' argument' + ('s' if count != 1 else '') + ', got ' + str(len(given) - 1))


def _deque_protocol(name, count):
    def decorate(function):
        def invoke(*given, **keywords):
            _deque_slot_check(name, count, given, keywords)
            return function(*given)
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
            raise TypeError("deque() got an unexpected keyword argument '" + key + "'")
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
    data = _deque_iter_start(iterator, owner, reverse)
    walker = iterator._walker
    for _ in range(min(max(0, index), len(data))):
        next(walker)


def _deque_iter_start(iterator, owner, reverse):
    core = _core_of(owner)
    core[3] = True
    data = core[0]
    iterator._owner = owner
    iterator._core = core
    iterator._state = core[2]
    iterator._size = len(data)
    iterator._walker = reversed(data) if reverse else iter(data)
    return data


class _deque_iterator:
    # The list behind a deque never changes identity, and any change of its
    # length bumps the owner's state, so while the state stands a native
    # walker over that list gives the items in order.  The items left are
    # the size at creation less those the walker has passed (none once the
    # iterator has noticed a mutation).
    __slots__ = ('_owner', '_core', '_state', '_walker', '_size')
    __module__ = 'collections'
    __native_type_name__ = '_collections._deque_iterator'

    def __init__(self, *args, **kwargs):
        owner, index = _deque_iter_args(args)
        _deque_iter_setup(self, owner, index, False)

    def __iter__(self):
        return self

    def __next__(self):
        if self._core[2] != self._state:
            self._size = 0
            raise RuntimeError('deque mutated during iteration')
        return next(self._walker)

    def _left(self):
        if not self._size:
            return 0
        parts = self._walker.__reduce__()
        return self._size - parts[2] if len(parts) > 2 else 0

    def __length_hint__(self):
        return self._left()

    def __reduce__(self):
        return (type(self), (self._owner, len(_data_of(self._owner)) - self._left()))


class _deque_reverse_iterator:
    __slots__ = ('_owner', '_core', '_state', '_walker', '_size')
    __module__ = 'collections'
    __native_type_name__ = '_collections._deque_reverse_iterator'

    def __init__(self, *args, **kwargs):
        owner, index = _deque_iter_args(args)
        _deque_iter_setup(self, owner, index, True)

    __iter__ = _deque_iterator.__iter__
    __length_hint__ = _deque_iterator.__length_hint__
    __reduce__ = _deque_iterator.__reduce__

    def __next__(self):
        if self._core[2] != self._state:
            if not self._left():
                raise StopIteration
            self._size = 0
            raise RuntimeError('deque mutated during iteration')
        return next(self._walker)

    def _left(self):
        if not self._size:
            return 0
        parts = self._walker.__reduce__()
        return parts[2] + 1 if len(parts) > 2 else 0


_MINIMUM = -_sys.maxsize - 1


# Items of these exact types compare without running any Python code, so a
# scan over them cannot change the deque and may be left to the list itself.
_PLAIN_KINDS = frozenset((int, float, bool, str, bytes, complex, type(None)))


# Containers of these exact types are read without running any Python code.
_BULK_KINDS = (list, tuple, range, str, bytes)


def _deque_plain(data, value):
    return type(value) in _PLAIN_KINDS and _PLAIN_KINDS.issuperset(map(type, data))


def _deque_slice_index(value):
    if not hasattr(type(value), '__index__'):
        raise TypeError('slice indices must be integers or have an __index__ method')
    return _index(value)


def _deque_index_value(position):
    # The index of a deque item, as the sequence slots read it.
    position = _index(position)
    if position > _sys.maxsize or position < -_sys.maxsize - 1:
        raise IndexError("cannot fit 'int' into an index-sized integer")
    return position


def _deque_fill(self, core, iterable):
    # deque.extend once the arguments are known to be right.
    data = core[0]
    limit = core[1]
    if iterable is self:
        iterable = list(data)
    if type(iterable) in _BULK_KINDS or type(iterable) is deque:
        # an empty source adds nothing, so live iterators stay valid
        if limit != 0 and len(iterable):
            data.extend(_data_of(iterable) if type(iterable) is deque else iterable)
            if limit is not None and len(data) > limit:
                del data[:len(data) - limit]
            core[2] += 1
        return
    if limit == 0:
        for value in iterable:
            pass
        return
    for value in iterable:
        data.append(value)
        if limit is not None and len(data) > limit:
            data.pop(0)
        core[2] += 1


class deque:
    """A list-like sequence optimized for data accesses near its endpoints."""
    __slots__ = ('_core', '__weakref__')
    __hash__ = None
    __module__ = 'collections'
    __native_type_name__ = 'collections.deque'
    __abc_tpflags__ = 32

    def __new__(cls=_MISSING, /, *args, **kwargs):
        if cls is not deque:
            if cls is _MISSING:
                raise TypeError('collections.deque.__new__(): not enough arguments')
            if not isinstance(cls, type):
                raise TypeError('collections.deque.__new__(X): X is not a type object (' + type(cls).__name__ + ')')
            if not issubclass(cls, deque):
                raise TypeError('collections.deque.__new__(' + cls.__name__ + '): ' + cls.__name__ + ' is not a subtype of collections.deque')
        owner = _object_new(cls)
        _core_put(owner, [[], None, 0, False])
        return owner

    def __init__(self=_MISSING, /, *args, **kwargs):
        if self is _MISSING:
            raise TypeError("descriptor '__init__' of 'collections.deque' object needs an argument")
        if not isinstance(self, deque):
            raise TypeError("descriptor '__init__' requires a 'collections.deque' object but received a '" + type(self).__name__ + "'")
        if kwargs or len(args) > 2:
            iterable, maxlen = _deque_args(args, kwargs)
        else:
            iterable = args[0] if args else _MISSING
            maxlen = args[1] if len(args) == 2 else None
        if maxlen is None:
            limit = None
        else:
            if not isinstance(maxlen, int):
                raise TypeError('an integer is required')
            limit = _deque_size(maxlen)
            if limit < 0:
                raise ValueError('maxlen must be non-negative')
        core = _core_of(self)
        core[1] = limit
        if limit is not None:
            core[3] = True
        if core[0]:
            del core[0][:]
            core[2] += 1
        if iterable is not _MISSING:
            _deque_fill(self, core, iterable)

    @property
    def maxlen(self):
        return _maxlen_of(self)

    @maxlen.setter
    def maxlen(self, value):
        raise AttributeError("attribute 'maxlen' of 'collections.deque' objects is not writable")

    @maxlen.deleter
    def maxlen(self):
        raise AttributeError("attribute 'maxlen' of 'collections.deque' objects is not writable")

    # The end operations are the hot path: each does its own cheap argument
    # test and falls back on _deque_check only to find which error to raise.

    def append(self=_MISSING, value=_MISSING, /, *extra, **keywords):
        if extra or keywords or value is _MISSING:
            _deque_check('append', 1, 1, _given(extra, self, value), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('append', 1, 1, _given(extra, self, value), keywords)
            raise
        data = core[0]
        data.append(value)
        if core[3]:
            core[2] += 1
            if core[1] is not None and len(data) > core[1]:
                data.pop(0)

    def appendleft(self=_MISSING, value=_MISSING, /, *extra, **keywords):
        if extra or keywords or value is _MISSING:
            _deque_check('appendleft', 1, 1, _given(extra, self, value), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('appendleft', 1, 1, _given(extra, self, value), keywords)
            raise
        data = core[0]
        data.insert(0, value)
        if core[3]:
            core[2] += 1
            if core[1] is not None and len(data) > core[1]:
                data.pop()

    def pop(self=_MISSING, /, *extra, **keywords):
        if extra or keywords:
            _deque_check('pop', 0, 0, _given(extra, self), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('pop', 0, 0, _given(extra, self), keywords)
            raise
        data = core[0]
        if not data:
            raise IndexError('pop from an empty deque')
        if core[3]:
            core[2] += 1
        return data.pop()

    def popleft(self=_MISSING, /, *extra, **keywords):
        if extra or keywords:
            _deque_check('popleft', 0, 0, _given(extra, self), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('popleft', 0, 0, _given(extra, self), keywords)
            raise
        data = core[0]
        if not data:
            raise IndexError('pop from an empty deque')
        if core[3]:
            core[2] += 1
        return data.pop(0)

    def extend(self=_MISSING, iterable=_MISSING, /, *extra, **keywords):
        if extra or keywords or iterable is _MISSING:
            _deque_check('extend', 1, 1, _given(extra, self, iterable), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('extend', 1, 1, _given(extra, self, iterable), keywords)
            raise
        _deque_fill(self, core, iterable)

    @_deque_method('extendleft', 1, 1)
    def extendleft(self, iterable, /):
        data = _data_of(self)
        if iterable is self:
            iterable = list(data)
        limit = _maxlen_of(self)
        if type(iterable) in _BULK_KINDS or type(iterable) is deque:
            if limit != 0 and len(iterable):
                items = list(_data_of(iterable) if type(iterable) is deque else iterable)
                items.reverse()
                data[:0] = items
                if limit is not None and len(data) > limit:
                    del data[limit:]
                _state_put(self, _state_of(self) + 1)
            return
        if limit == 0:
            for value in iterable:
                pass
            return
        for value in iterable:
            data.insert(0, value)
            if limit is not None and len(data) > limit:
                data.pop()
            _state_put(self, _state_of(self) + 1)

    @_deque_method('clear', 0, 0)
    def clear(self):
        data = _data_of(self)
        if data:
            del data[:]
            _state_put(self, _state_of(self) + 1)

    def rotate(self=_MISSING, n=1, /, *extra, **keywords):
        if extra or keywords:
            _deque_check('rotate', 0, 1, _given(extra, self, n), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('rotate', 0, 1, _given(extra, self, n), keywords)
            raise
        if type(n) is not int or not _MINIMUM <= n <= _sys.maxsize:
            n = _deque_size(n)
        data = core[0]
        size = len(data)
        if size > 1:
            core[2] += 1
            n %= size
            if n:
                data[:] = data[-n:] + data[:-n]

    @_deque_method('reverse', 0, 0)
    def reverse(self):
        _data_of(self).reverse()

    @_deque_method('count', 1, 1)
    def count(self, value, /):
        result = 0
        data = _data_of(self)
        if _deque_plain(data, value):
            return data.count(value)
        state = _state_of(self)
        for position in range(len(data)):
            item = data[position]
            if item is value or item == value:
                result += 1
            if state != _state_of(self):
                raise RuntimeError('deque mutated during iteration')
        return result

    def index(self=_MISSING, value=_MISSING, start=_MISSING, stop=_MISSING, /, *extra, **keywords):
        if extra or keywords or value is _MISSING:
            _deque_check('index', 1, 3, _given(extra, self, value, start, stop), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_check('index', 1, 3, _given(extra, self, value, start, stop), keywords)
            raise
        data = core[0]
        if start is _MISSING:
            start = 0
        elif type(start) is not int:
            start = _deque_slice_index(start)
        if stop is _MISSING:
            stop = _sys.maxsize
        elif type(stop) is not int:
            stop = _deque_slice_index(stop)
        size = len(data)
        if start < 0:
            start = max(0, start + size)
        if stop < 0:
            stop = max(0, stop + size)
        if stop > size:
            stop = size
        part = data[start:stop]
        if _deque_plain(part, value):
            if value in part:
                return start + part.index(value)
            raise ValueError('deque.index(x): x not in deque')
        core[3] = True
        state = core[2]
        for position in range(start, stop):
            item = data[position]
            found = item is value or item == value
            if found:
                return position
            if state != core[2]:
                raise RuntimeError('deque mutated during iteration')
        raise ValueError('deque.index(x): x not in deque')

    @_deque_method('insert', 2, 2)
    def insert(self, position, value, /):
        position = _deque_size(position)
        data = _data_of(self)
        if _maxlen_of(self) is not None and len(data) == _maxlen_of(self):
            raise IndexError('deque already at its maximum size')
        data.insert(position, value)
        _state_put(self, _state_of(self) + 1)

    @_deque_method('remove', 1, 1)
    def remove(self, value, /):
        data = _data_of(self)
        if _deque_plain(data, value):
            if value not in data:
                raise ValueError('deque.remove(x): x not in deque')
            data.pop(data.index(value))
            _state_put(self, _state_of(self) + 1)
            return
        state = _state_of(self)
        for position in range(len(data)):
            item = data[position]
            found = item is value or item == value
            if state != _state_of(self):
                raise IndexError('deque mutated during iteration')
            if found:
                data.pop(position)
                _state_put(self, _state_of(self) + 1)
                return
        raise ValueError('deque.remove(x): x not in deque')

    @_deque_method('copy', 0, 0)
    def copy(self):
        cls = type(self)
        result = cls(self) if _maxlen_of(self) is None else cls(self, _maxlen_of(self))
        if not isinstance(result, deque):
            raise TypeError(cls.__name__ + '() must return a deque, not ' + type(result).__name__)
        return result

    @_deque_method('__copy__', 0, 0)
    def __copy__(self):
        return deque.copy(self)

    def __len__(self=_MISSING, /, *extra, **keywords):
        if extra or keywords:
            _deque_slot_check('__len__', 0, _given(extra, self), keywords)
        try:
            return len(_core_of(self)[0])
        except Exception:
            _deque_slot_check('__len__', 0, _given(extra, self), keywords)
            raise

    def __iter__(self=_MISSING, /, *extra, **keywords):
        if extra or keywords:
            _deque_slot_check('__iter__', 0, _given(extra, self), keywords)
        try:
            _core_of(self)
        except Exception:
            _deque_slot_check('__iter__', 0, _given(extra, self), keywords)
            raise
        iterator = _object_new(_deque_iterator)
        _deque_iter_start(iterator, self, False)
        return iterator

    def __reversed__(self=_MISSING, /, *extra, **keywords):
        if extra or keywords:
            _deque_check('__reversed__', 0, 0, _given(extra, self), keywords)
        try:
            _core_of(self)
        except Exception:
            _deque_check('__reversed__', 0, 0, _given(extra, self), keywords)
            raise
        iterator = _object_new(_deque_reverse_iterator)
        _deque_iter_start(iterator, self, True)
        return iterator

    def __getitem__(self=_MISSING, position=_MISSING, /, *extra, **keywords):
        if extra or keywords or position is _MISSING:
            _deque_slot_check('__getitem__', 1, _given(extra, self, position), keywords)
        try:
            data = _core_of(self)[0]
        except Exception:
            _deque_slot_check('__getitem__', 1, _given(extra, self, position), keywords)
            raise
        if type(position) is not int:
            if not hasattr(type(position), '__index__'):
                raise TypeError("sequence index must be integer, not '" + type(position).__name__ + "'")
            position = _deque_index_value(position)
        try:
            return data[position]
        except IndexError:
            if -_sys.maxsize - 1 <= position <= _sys.maxsize:
                raise IndexError('deque index out of range') from None
            raise IndexError("cannot fit 'int' into an index-sized integer") from None

    def __setitem__(self=_MISSING, position=_MISSING, value=_MISSING, /, *extra, **keywords):
        if extra or keywords or value is _MISSING:
            _deque_slot_check('__setitem__', 2, _given(extra, self, position, value), keywords)
        try:
            data = _core_of(self)[0]
        except Exception:
            _deque_slot_check('__setitem__', 2, _given(extra, self, position, value), keywords)
            raise
        if type(position) is int and -len(data) <= position < len(data):
            data[position] = value
            return
        position = _deque_index_value(position)
        if not -len(data) <= position < len(data):
            raise IndexError('deque index out of range')
        data[position] = value

    def __delitem__(self=_MISSING, position=_MISSING, /, *extra, **keywords):
        if extra or keywords or position is _MISSING:
            _deque_slot_check('__delitem__', 1, _given(extra, self, position), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_slot_check('__delitem__', 1, _given(extra, self, position), keywords)
            raise
        data = core[0]
        if type(position) is not int:
            position = _deque_index_value(position)
        if not -len(data) <= position < len(data):
            raise IndexError('deque index out of range')
        data.pop(position)
        core[2] += 1

    def __contains__(self=_MISSING, value=_MISSING, /, *extra, **keywords):
        if extra or keywords or value is _MISSING:
            _deque_slot_check('__contains__', 1, _given(extra, self, value), keywords)
        try:
            core = _core_of(self)
        except Exception:
            _deque_slot_check('__contains__', 1, _given(extra, self, value), keywords)
            raise
        data = core[0]
        if _deque_plain(data, value):
            return value in data
        core[3] = True
        state = core[2]
        for position in range(len(data)):
            item = data[position]
            found = item is value or item == value
            if found:
                return True
            if state != core[2]:
                raise RuntimeError('deque mutated during iteration')
        return False

    @_deque_protocol('__eq__', 1)
    def __eq__(self, other):
        if not isinstance(other, deque):
            return NotImplemented
        if self is other:
            return True
        if len(_data_of(self)) != len(_data_of(other)):
            return False
        return deque._compare(self, other, _eq)

    @_deque_protocol('__ne__', 1)
    def __ne__(self, other):
        if not isinstance(other, deque):
            return NotImplemented
        if self is other:
            return False
        if len(_data_of(self)) != len(_data_of(other)):
            return True
        return deque._compare(self, other, _ne)

    def _compare(self, other, operation):
        if not isinstance(other, deque):
            return NotImplemented
        one, two = _data_of(self), _data_of(other)
        if _deque_plain(one, None) and _deque_plain(two, None):
            return operation(one, two)
        left, right = iter(self), iter(other)
        exhausted = object()
        while True:
            a = next(left, exhausted)
            b = next(right, exhausted)
            if a is exhausted or b is exhausted:
                return operation(a is not exhausted, b is not exhausted)
            if a is b or a == b:
                continue
            return bool(operation(a, b))

    @_deque_protocol('__lt__', 1)
    def __lt__(self, other):
        return deque._compare(self, other, _lt)

    @_deque_protocol('__le__', 1)
    def __le__(self, other):
        return deque._compare(self, other, _le)

    @_deque_protocol('__gt__', 1)
    def __gt__(self, other):
        return deque._compare(self, other, _gt)

    @_deque_protocol('__ge__', 1)
    def __ge__(self, other):
        return deque._compare(self, other, _ge)

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
        if not hasattr(type(times), '__index__'):
            raise TypeError("can't multiply sequence by non-int of type '" + type(times).__name__ + "'")
        times = _index(times)
        if times > _sys.maxsize or times < -_sys.maxsize - 1:
            raise OverflowError("cannot fit 'int' into an index-sized integer")
        data = _data_of(self)
        if not data or times == 1:
            return self
        if times <= 0:
            deque.clear(self)
            return self
        size = len(data)
        if size == 1:
            limit = _maxlen_of(self)
            if limit is not None:
                times = min(times, limit)
            _state_put(self, _state_of(self) + 1)
            item = data[0]
            for _ in range(times - 1):
                data.append(item)
            return self
        if size > _sys.maxsize // times:
            raise MemoryError
        old = data[:]
        if _maxlen_of(self) is not None:
            times = min(times, (_maxlen_of(self) + size - 1) // size)
        for _ in range(times - 1):
            deque.extend(self, old)
        return self

    @_deque_protocol('__repr__', 0)
    @_recursive_repr('[...]')
    def __repr__(self):
        result = type(self).__name__ + '(' + repr(list(self))
        if _maxlen_of(self) is not None:
            result += ', maxlen=' + str(_maxlen_of(self))
        return result + ')'

    @_deque_method('__reduce__', 0, 0)
    def __reduce__(self):
        args = () if _maxlen_of(self) is None else ((), _maxlen_of(self))
        for cls in type(self).__mro__:
            if cls is object:
                break
            if '__getstate__' in cls.__dict__:
                return (type(self), args, self.__getstate__(), iter(self))
        state = __reduce_native__(self, True) or None
        slots = {}
        for cls in type(self).__mro__:
            if cls is deque or cls is object:
                continue
            names = cls.__dict__.get('__slots__', ())
            if isinstance(names, str):
                names = (names,)
            for name in names:
                if name in ('__dict__', '__weakref__'):
                    continue
                if name.startswith('__') and not name.endswith('__'):
                    prefix = cls.__name__.lstrip('_')
                    if prefix:
                        name = '_' + prefix + name
                try:
                    slots[name] = getattr(self, name)
                except AttributeError:
                    pass
        if slots:
            state = (state, slots)
        return (type(self), args, state, iter(self))

    def __class_getitem__(*given, **keywords):
        if keywords:
            raise TypeError('deque.__class_getitem__() takes no keyword arguments')
        if len(given) != 2:
            raise TypeError('deque.__class_getitem__() takes exactly one argument (' + str(len(given) - 1) + ' given)')
        from types import GenericAlias
        return GenericAlias(*given)


_core_of, _core_put = _deque_bind('_core')


def _data_of(owner):
    return _core_of(owner)[0]


def _maxlen_of(owner):
    return _core_of(owner)[1]


def _state_of(owner):
    # Reading the state is looking at the deque: from now on mutations count.
    core = _core_of(owner)
    core[3] = True
    return core[2]


def _state_put(owner, value):
    _core_of(owner)[2] = value


_repr_factories = set()


class defaultdict(dict):
    __module__ = 'collections'
    # The factory is kept in a slot the class alone names, apart from any
    # attribute a subclass may define, the way the reference keeps it in
    # a member of its own.
    __slots__ = ('_defaultdict__factory',)
    __module__ = 'collections'

    __module__ = 'collections'
    def __new__(cls, default_factory=None, *args, **kwargs):
        return super().__new__(cls)

    def __init__(self, default_factory=None, *args, **kwargs):
        if default_factory is not None and not callable(default_factory):
            raise TypeError('first argument must be callable or None')
        self._defaultdict__factory = default_factory
        super().__init__(*args, **kwargs)

    @property
    def default_factory(self):
        return self._defaultdict__factory

    @default_factory.setter
    def default_factory(self, factory):
        self._defaultdict__factory = factory

    @default_factory.deleter
    def default_factory(self):
        self._defaultdict__factory = None

    def __getitem__(self, key):
        return dict.__getitem__(self, key)

    def __missing__(self, key):
        factory = self._defaultdict__factory
        if factory is None:
            raise KeyError(key)
        value = factory()
        # CPython 3.14 uses PyDict_SetDefaultRef: a recursive factory's
        # insertion wins, and a subclass's assignment hook is bypassed.
        return dict.setdefault(self, key, value)

    def __repr__(self):
        inside = dict.__repr__(self)
        factory = self._defaultdict__factory
        if factory is None:
            shown = 'None'
        else:
            marker = id(factory)
            if marker in _repr_factories:
                shown = '...'
            else:
                _repr_factories.add(marker)
                try:
                    shown = repr(factory)
                finally:
                    _repr_factories.discard(marker)
        return '%s(%s, %s)' % (type(self).__name__, shown, inside)

    def copy(self):
        return type(self)(self._defaultdict__factory, self)

    def __copy__(self):
        return self.copy()

    def __or__(self, other):
        if not isinstance(other, dict):
            return NotImplemented
        new = type(self)(self._defaultdict__factory, self)
        dict.update(new, other)
        return new

    def __ror__(self, other):
        if not isinstance(other, dict):
            return NotImplemented
        new = type(self)(self._defaultdict__factory, other)
        dict.update(new, self)
        return new

    def __reduce__(self):
        if self._defaultdict__factory is None:
            args = ()
        else:
            args = (self._defaultdict__factory,)
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
        if self._index < 0:
            raise IndexError('tuple index out of range')
        return tuple.__getitem__(instance, self._index)

    def __set__(self, instance, value):
        raise AttributeError('readonly attribute')

    def __delete__(self, instance):
        raise AttributeError('readonly attribute')


# An OrderedDict iterator a run may store and read back: the C view
# iterators are picklable where a generator is not.
class _OrderedDictIterator:
    __slots__ = ('_mapping', '_kind', '_back', '_keys', '_at', '_version')

    def __init__(self, mapping, kind, back=False):
        self._mapping = mapping
        self._kind = kind
        self._back = back
        self._keys = []
        root = object.__getattribute__(mapping, '_OrderedDict__root')
        current = root.next
        while current is not root:
            self._keys.append(current.key)
            current = current.next
        if back:
            self._keys.reverse()
        self._at = 0
        self._version = mapping._version

    def __iter__(self):
        return self

    def __next__(self):
        if self._mapping._version != self._version:
            raise RuntimeError('OrderedDict mutated during iteration')
        if self._at >= len(self._keys):
            raise StopIteration
        key = self._keys[self._at]
        self._at += 1
        if self._kind == 'k':
            return key
        value = dict.__getitem__(self._mapping, key)
        if self._kind == 'v':
            return value
        return (key, value)

    def __length_hint__(self):
        return len(self._keys) - self._at

    def __reduce__(self):
        if self._mapping._version != self._version:
            raise RuntimeError('OrderedDict mutated during iteration')
        remaining = []
        for key in self._keys[self._at:]:
            if self._kind == 'k':
                remaining.append(key)
            else:
                value = dict.__getitem__(self._mapping, key)
                remaining.append(value if self._kind == 'v' else (key, value))
        return (iter, (remaining,))

    def __setstate__(self, state):
        self._at, self._version = state


def _native_ordered_dict(base):
    "The C OrderedDict: the pure class plus the mutation checks the C one makes."
    from _collections_abc import KeysView, ValuesView, ItemsView
    from operator import eq as _eq

    class _KeysView(KeysView):
        def __iter__(self):
            return _OrderedDictIterator(self._mapping, 'k')

        def __reversed__(self):
            return _OrderedDictIterator(self._mapping, 'k', True)

    class _ItemsView(ItemsView):
        def __iter__(self):
            return _OrderedDictIterator(self._mapping, 'i')

        def __reversed__(self):
            return _OrderedDictIterator(self._mapping, 'i', True)

    class _ValuesView(ValuesView):
        def __iter__(self):
            return _OrderedDictIterator(self._mapping, 'v')

        def __reversed__(self):
            return _OrderedDictIterator(self._mapping, 'v', True)

    marker = object()

    class OrderedDict(base):
        __module__ = 'collections'
        __slots__ = ('_version',)
        __module__ = 'collections'

        def __new__(cls, /, *args, **kwds):
            self = base.__new__(cls, *args, **kwds)
            self._version = 0
            return self

        def _changed(self):
            self._version += 1

        def __setitem__(self, key, value):
            fresh = key not in self
            base.__setitem__(self, key, value)
            if fresh:
                self._changed()

        def __delitem__(self, key):
            base.__delitem__(self, key)
            self._changed()

        def clear(self):
            base.clear(self)
            self._changed()

        def popitem(self, last=True):
            pair = base.popitem(self, last)
            self._changed()
            return pair

        def pop(self, key, default=marker):
            if key in self:
                value = base.pop(self, key)
                self._changed()
                return value
            if default is marker:
                raise KeyError(key)
            return default

        def move_to_end(self, key, last=True):
            base.move_to_end(self, key, last)
            self._changed()

        def setdefault(self, key, default=None):
            fresh = key not in self
            value = base.setdefault(self, key, default)
            if fresh:
                self._changed()
            return value

        def keys(self):
            return _KeysView(self)

        def items(self):
            return _ItemsView(self)

        def values(self):
            return _ValuesView(self)

        def __getstate__(self):
            state = object.__getstate__(self)
            if isinstance(state, tuple) and len(state) == 2:
                visible, slots = state
                slots = dict(slots)
                slots.pop('_version', None)
                return (visible, slots) if slots else visible
            return state

        def __eq__(self, other):
            if not isinstance(other, base):
                return dict.__eq__(self, other)
            version = self._version
            plain = dict.__eq__(self, other)
            changed = self._version != version
            if not plain and not changed:
                return False
            # The ordered pass still runs when a key changed the map during
            # the plain comparison, so every key the reference would compare
            # is compared; the answer is inequality either way.
            ordered = all(map(_eq, self, other))
            if changed:
                return False
            return ordered

        def __iter__(self):
            return _OrderedDictIterator(self, 'k')

        def __reversed__(self):
            return _OrderedDictIterator(self, 'k', True)

    OrderedDict.__module__ = 'collections'
    OrderedDict.__qualname__ = 'OrderedDict'
    OrderedDict.__name__ = 'OrderedDict'
    return OrderedDict


def __getattr__(name):
    if name == 'OrderedDict':
        from collections import OrderedDict as pure
        built = _native_ordered_dict(pure)
        globals()['OrderedDict'] = built
        return built
    raise AttributeError('module %r has no attribute %r' % (__name__, name))

__seal_class(_deque_iterator)
__seal_class(_deque_reverse_iterator)
