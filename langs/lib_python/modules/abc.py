# Abstract base classes. ABCMeta is a real metaclass: it refuses to make
# a thing of a class that still carries an abstract method nobody has
# answered, abstractmethod is what marks one, and register claims a
# class for a kind it does not stand under.
#
# isinstance and issubclass ask a class's metaclass before they read the
# class a thing was built from, so the claims below are honoured: a
# claim made for a kind is a claim for every kind that kind stands
# under, and a kind may speak for whatever answers to the right names
# through __subclasshook__.

# Which classes have been claimed for which kind: each entry holds the
# kind and the classes claimed for it. Classes are told apart by which
# one they are rather than by name, so the claims are kept as pairs.
_claimed = []

# The questions under way, so that a kind whose claims lead back to it
# does not send the same question round for ever.
_asking = []


def _claims_for(kind):
    for entry in _claimed:
        if entry[0] is kind:
            return entry[1]
    return None


def _stands_under(subclass, cls):
    # Whether subclass really inherits cls. Read from the line of
    # forebears rather than asked of issubclass, which would come
    # straight back here.
    if subclass is cls:
        return True
    line = getattr(subclass, '__mro__', None)
    if line is None:
        return False
    for forebear in line:
        if forebear is cls:
            return True
    return False


def _counts_as(cls, subclass):
    # The kind speaks first, where it has something to say about what a
    # class answers to.
    hook = getattr(cls, '__subclasshook__', None)
    if hook is not None:
        told = hook(subclass)
        if told is not NotImplemented:
            return bool(told)
    if _stands_under(subclass, cls):
        return True
    # A claim made for cls, or for any kind standing under cls, counts:
    # claiming int for Integral claims it for Number as well.
    for entry in _claimed:
        if entry[0] is cls or _stands_under(entry[0], cls):
            for claimed in entry[1]:
                if claimed is subclass or _stands_under(subclass, claimed):
                    return True
                # A claim may name a kind the reader cannot weigh one
                # class against; such a claim holds for the very class
                # it names and for nothing else.
                try:
                    if issubclass(subclass, claimed):
                        return True
                except Exception:
                    pass
    return False


class ABCMeta(type):
    def __call__(cls, *args, **kwargs):
        missing = []
        for name in dir(cls):
            if getattr(getattr(cls, name, None), '__isabstractmethod__', False):
                missing.append(name)
        if missing:
            missing.sort()
            raise TypeError("Can't instantiate abstract class %s with abstract method%s %s"
                            % (cls.__name__, 's' if len(missing) > 1 else '', ', '.join(missing)))
        return super().__call__(*args, **kwargs)

    def register(cls, subclass):
        claimed = _claims_for(cls)
        if claimed is None:
            claimed = []
            _claimed.append((cls, claimed))
        for already in claimed:
            if already is subclass:
                return subclass
        claimed.append(subclass)
        return subclass

    def __instancecheck__(cls, instance):
        return cls.__subclasscheck__(type(instance))

    def __subclasscheck__(cls, subclass):
        for pair in _asking:
            if pair[0] is cls and pair[1] is subclass:
                return False
        _asking.append((cls, subclass))
        try:
            return _counts_as(cls, subclass)
        finally:
            _asking.pop()


class ABC(metaclass=ABCMeta):
    @classmethod
    def __subclasshook__(cls, subclass):
        return NotImplemented


def abstractmethod(function):
    function.__isabstractmethod__ = True
    return function
