# The container kinds, as classes to inherit from and to ask after. The
# mixin methods below are real: a class that gives the few abstract
# methods its kind names gets the rest of the kind's behaviour from
# here.
#
# Stub: register records the claim and answers with the class it was
# given, as CPython's does, but isinstance cannot honour it. The reader
# answers isinstance from the class a thing was built from, and offers
# no instance-check hook for a class to speak for kinds it does not
# stand above, so isinstance(x, Mapping) is true only where the class of
# x really inherits Mapping.
from abc import ABC, abstractmethod

__all__ = ['Hashable', 'Sized', 'Container', 'Callable', 'Iterable',
           'Iterator', 'Reversible', 'Generator', 'Collection',
           'Sequence', 'MutableSequence', 'Set', 'MutableSet',
           'Mapping', 'MutableMapping', 'MappingView', 'KeysView',
           'ValuesView', 'ItemsView']

# Which classes have been claimed for which kind, by the kind's name.
_claimed = {}


# The mixin comparisons below read the other operand by what it answers
# to rather than by the kind it was claimed for, since a claim made
# through register cannot be read back through isinstance here.
def _a_set(value):
    return isinstance(value, (set, frozenset)) or isinstance(value, Set)


def _a_mapping(value):
    return isinstance(value, dict) or isinstance(value, Mapping) or hasattr(value, 'keys')


def _a_walk(value):
    # Whether a value can be walked at all, which is all the set
    # operations below ask of the other operand.
    try:
        iter(value)
    except TypeError:
        return False
    return True


class _Kind(ABC):
    # A kind every container kind below stands on, for the one thing
    # they share: the claim a class makes to belong to the kind.
    @classmethod
    def register(cls, subclass):
        table = _claimed
        name = cls.__name__
        if name not in table:
            table[name] = []
        if subclass not in table[name]:
            table[name].append(subclass)
        return subclass

    @classmethod
    def _registered(cls):
        table = _claimed
        name = cls.__name__
        if name not in table:
            return []
        return list(table[name])


class Hashable(_Kind):
    @abstractmethod
    def __hash__(self):
        return 0


class Sized(_Kind):
    @abstractmethod
    def __len__(self):
        raise NotImplementedError('__len__')


class Container(_Kind):
    @abstractmethod
    def __contains__(self, value):
        raise NotImplementedError('__contains__')


class Callable(_Kind):
    @abstractmethod
    def __call__(self, *args, **keywords):
        raise NotImplementedError('__call__')


class Iterable(_Kind):
    @abstractmethod
    def __iter__(self):
        raise NotImplementedError('__iter__')


class Iterator(Iterable):
    @abstractmethod
    def __next__(self):
        raise StopIteration

    def __iter__(self):
        return self


class Reversible(Iterable):
    @abstractmethod
    def __reversed__(self):
        raise NotImplementedError('__reversed__')


class Generator(Iterator):
    def __next__(self):
        return self.send(None)

    @abstractmethod
    def send(self, value):
        raise StopIteration

    @abstractmethod
    def throw(self, error, value=None, traceback=None):
        raise error

    def close(self):
        try:
            self.throw(GeneratorExit)
        except GeneratorExit:
            return
        except StopIteration:
            return
        raise RuntimeError('generator ignored GeneratorExit')


class Collection(Sized, Iterable, Container):
    pass


# A walk over anything that answers to whole numbers from zero up.
class _IndexWalk:
    def __init__(self, sequence):
        self._sequence = sequence
        self._place = 0

    def __iter__(self):
        return self

    def __next__(self):
        try:
            value = self._sequence[self._place]
        except IndexError:
            raise StopIteration
        self._place = self._place + 1
        return value


class _BackwardWalk:
    def __init__(self, sequence):
        self._sequence = sequence
        self._place = len(sequence) - 1

    def __iter__(self):
        return self

    def __next__(self):
        if self._place < 0:
            raise StopIteration
        value = self._sequence[self._place]
        self._place = self._place - 1
        return value


class Sequence(Reversible, Collection):
    @abstractmethod
    def __getitem__(self, index):
        raise IndexError('index out of range')

    @abstractmethod
    def __len__(self):
        raise NotImplementedError('__len__')

    def __iter__(self):
        return _IndexWalk(self)

    def __reversed__(self):
        return _BackwardWalk(self)

    def __contains__(self, value):
        for held in self:
            if held is value or held == value:
                return True
        return False

    def index(self, value, start=0, stop=None):
        end = len(self) if stop is None else stop
        if start < 0:
            start = max(len(self) + start, 0)
        if end < 0:
            end = max(len(self) + end, 0)
        place = start
        while place < end:
            held = self[place]
            if held is value or held == value:
                return place
            place = place + 1
        raise ValueError(repr(value) + ' is not in sequence')

    def count(self, value):
        total = 0
        for held in self:
            if held is value or held == value:
                total = total + 1
        return total


class MutableSequence(Sequence):
    @abstractmethod
    def __setitem__(self, index, value):
        raise IndexError('index out of range')

    @abstractmethod
    def __delitem__(self, index):
        raise IndexError('index out of range')

    @abstractmethod
    def insert(self, index, value):
        raise NotImplementedError('insert')

    def append(self, value):
        self.insert(len(self), value)

    def extend(self, values):
        if values is self:
            values = list(values)
        for value in values:
            self.append(value)

    def reverse(self):
        count = len(self)
        place = 0
        while place < count // 2:
            other = count - place - 1
            held = self[place]
            self[place] = self[other]
            self[other] = held
            place = place + 1

    def pop(self, index=-1):
        value = self[index]
        del self[index]
        return value

    def remove(self, value):
        del self[self.index(value)]

    def clear(self):
        while True:
            try:
                self.pop()
            except IndexError:
                return

    def __iadd__(self, values):
        self.extend(values)
        return self


class Set(Collection):
    @abstractmethod
    def __contains__(self, value):
        raise NotImplementedError('__contains__')

    @abstractmethod
    def __iter__(self):
        raise NotImplementedError('__iter__')

    @abstractmethod
    def __len__(self):
        raise NotImplementedError('__len__')

    # What a set operation below builds its answer as. A kind whose
    # maker does not take a walk of values gives its own.
    @classmethod
    def _from_iterable(cls, values):
        return cls(values)

    def __le__(self, other):
        if not _a_set(other):
            return NotImplemented
        if len(self) > len(other):
            return False
        for value in self:
            if value not in other:
                return False
        return True

    def __lt__(self, other):
        if not _a_set(other):
            return NotImplemented
        return len(self) < len(other) and self.__le__(other)

    def __ge__(self, other):
        if not _a_set(other):
            return NotImplemented
        if len(self) < len(other):
            return False
        for value in other:
            if value not in self:
                return False
        return True

    def __gt__(self, other):
        if not _a_set(other):
            return NotImplemented
        return len(self) > len(other) and self.__ge__(other)

    def __eq__(self, other):
        return _a_set(other) and len(self) == len(other) and self.__le__(other)

    def isdisjoint(self, other):
        for value in other:
            if value in self:
                return False
        return True

    def __and__(self, other):
        if not _a_walk(other):
            return NotImplemented
        kept = []
        for value in other:
            if value in self:
                kept.append(value)
        return self._from_iterable(kept)

    def __rand__(self, other):
        return self.__and__(other)

    def __or__(self, other):
        if not _a_walk(other):
            return NotImplemented
        gathered = list(self)
        for value in other:
            gathered.append(value)
        return self._from_iterable(gathered)

    def __ror__(self, other):
        return self.__or__(other)

    def __sub__(self, other):
        if not _a_set(other):
            if not _a_walk(other):
                return NotImplemented
            other = self._from_iterable(other)
        kept = []
        for value in self:
            if value not in other:
                kept.append(value)
        return self._from_iterable(kept)

    def __rsub__(self, other):
        if not _a_set(other):
            if not _a_walk(other):
                return NotImplemented
            other = self._from_iterable(other)
        kept = []
        for value in other:
            if value not in self:
                kept.append(value)
        return self._from_iterable(kept)

    def __xor__(self, other):
        if not _a_set(other):
            if not _a_walk(other):
                return NotImplemented
            other = self._from_iterable(other)
        return (self - other) | (other - self)

    def __rxor__(self, other):
        return self.__xor__(other)


class MutableSet(Set):
    @abstractmethod
    def add(self, value):
        raise NotImplementedError('add')

    @abstractmethod
    def discard(self, value):
        raise NotImplementedError('discard')

    def remove(self, value):
        if value not in self:
            raise KeyError(value)
        self.discard(value)

    def pop(self):
        for value in self:
            self.discard(value)
            return value
        raise KeyError('pop from an empty set')

    def clear(self):
        while True:
            try:
                self.pop()
            except KeyError:
                return

    def __ior__(self, other):
        for value in other:
            self.add(value)
        return self

    def __iand__(self, other):
        for value in list(self):
            if value not in other:
                self.discard(value)
        return self

    def __ixor__(self, other):
        if other is self:
            self.clear()
            return self
        for value in list(other):
            if value in self:
                self.discard(value)
            else:
                self.add(value)
        return self

    def __isub__(self, other):
        if other is self:
            self.clear()
            return self
        for value in list(other):
            self.discard(value)
        return self


class Mapping(Collection):
    @abstractmethod
    def __getitem__(self, key):
        raise KeyError(key)

    @abstractmethod
    def __iter__(self):
        raise NotImplementedError('__iter__')

    @abstractmethod
    def __len__(self):
        raise NotImplementedError('__len__')

    def get(self, key, default=None):
        try:
            return self[key]
        except KeyError:
            return default

    def __contains__(self, key):
        try:
            self[key]
        except KeyError:
            return False
        return True

    def keys(self):
        return KeysView(self)

    def items(self):
        return ItemsView(self)

    def values(self):
        return ValuesView(self)

    def __eq__(self, other):
        if not _a_mapping(other):
            return False
        if len(self) != len(other):
            return False
        for key in self:
            if key not in other or self[key] != other[key]:
                return False
        return True

    def __ne__(self, other):
        return not self.__eq__(other)


class MutableMapping(Mapping):
    @abstractmethod
    def __setitem__(self, key, value):
        raise NotImplementedError('__setitem__')

    @abstractmethod
    def __delitem__(self, key):
        raise KeyError(key)

    def pop(self, key, *default):
        try:
            value = self[key]
        except KeyError:
            if default:
                return default[0]
            raise
        del self[key]
        return value

    def popitem(self):
        for key in self:
            value = self[key]
            del self[key]
            return (key, value)
        raise KeyError('popitem from an empty mapping')

    def clear(self):
        while True:
            try:
                self.popitem()
            except KeyError:
                return

    def update(self, other=None, **keywords):
        if other is not None:
            if isinstance(other, dict) or isinstance(other, Mapping):
                for key in list(other):
                    self[key] = other[key]
            elif hasattr(other, 'keys'):
                for key in list(other.keys()):
                    self[key] = other[key]
            else:
                for key, value in other:
                    self[key] = value
        for key in list(keywords):
            self[key] = keywords[key]

    def setdefault(self, key, default=None):
        try:
            return self[key]
        except KeyError:
            self[key] = default
        return default


class MappingView(Sized):
    def __init__(self, mapping):
        self._mapping = mapping

    def __len__(self):
        return len(self._mapping)

    def __repr__(self):
        return type(self).__name__ + '(' + repr(self._mapping) + ')'


class KeysView(MappingView, Set):
    @classmethod
    def _from_iterable(cls, values):
        return set(values)

    def __contains__(self, key):
        return key in self._mapping

    def __iter__(self):
        return iter(self._mapping)


class ValuesView(MappingView, Collection):
    def __contains__(self, value):
        for key in self._mapping:
            held = self._mapping[key]
            if held is value or held == value:
                return True
        return False

    def __iter__(self):
        return _IndexWalk(_ValueList(self._mapping))


class _ValueList:
    # The values of a mapping, read by place, so the walk above serves.
    def __init__(self, mapping):
        self._keys = list(mapping)
        self._mapping = mapping

    def __getitem__(self, place):
        return self._mapping[self._keys[place]]

    def __len__(self):
        return len(self._keys)


class ItemsView(MappingView, Set):
    @classmethod
    def _from_iterable(cls, values):
        return set(values)

    def __contains__(self, pair):
        key, value = pair
        try:
            held = self._mapping[key]
        except KeyError:
            return False
        return held is value or held == value

    def __iter__(self):
        return _IndexWalk(_PairList(self._mapping))


class _PairList:
    def __init__(self, mapping):
        self._keys = list(mapping)
        self._mapping = mapping

    def __getitem__(self, place):
        key = self._keys[place]
        return (key, self._mapping[key])

    def __len__(self):
        return len(self._keys)


# The builtin containers, claimed for the kinds they answer to, so the
# registry reads as CPython's does.
Sequence.register(tuple)
Sequence.register(str)
Sequence.register(range)
MutableSequence.register(list)
Set.register(frozenset)
MutableSet.register(set)
MutableMapping.register(dict)
Hashable.register(int)
Hashable.register(float)
Hashable.register(str)
Hashable.register(tuple)
