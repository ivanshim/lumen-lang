# Abstract base classes. ABCMeta is a real metaclass: it refuses to make
# a thing of a class that still carries an abstract method nobody has
# answered, and abstractmethod is what marks one.
#
# Stub: register records the claim and answers with the class it was
# given, as CPython's does, but isinstance and issubclass cannot honour
# it. They read a thing from the class it was built from, and offer no
# hook for a class to speak for kinds it does not stand above.

# Which classes have been claimed for which kind, by the kind's name.
_claimed = {}


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
        table = _claimed
        name = cls.__name__
        if name not in table:
            table[name] = []
        if subclass not in table[name]:
            table[name].append(subclass)
        return subclass


class ABC(metaclass=ABCMeta):
    pass


def abstractmethod(function):
    function.__isabstractmethod__ = True
    return function
