# Runtime ABC bridge. Keep the checks in abc's module namespace, as the
# native ABC implementation does when it calls Python subclass hooks.
from _weakrefset import WeakSet as _ABCWeakSet

# Consult the positive cache, then check the apparent and actual instance types.
def _runtime_abc_instancecheck(cls, instance):
    try:
        apparent = instance.__class__
    except AttributeError:
        apparent = type(instance)
    if apparent in cls._abc_cache:
        return True
    actual = type(instance)
    if actual is apparent:
        if (cls._abc_negative_cache_version == ABCMeta._abc_invalidation_counter
                and apparent in cls._abc_negative_cache):
            return False
        return cls.__subclasscheck__(apparent)
    return cls.__subclasscheck__(apparent) or cls.__subclasscheck__(actual)

# Invalidate negative results on registration and apply the hook before ancestry.
def _runtime_abc_subclasscheck(cls, candidate):
    if not isinstance(candidate, type):
        raise TypeError('issubclass() arg 1 must be a class')
    if candidate in cls._abc_cache:
        return True
    version = ABCMeta._abc_invalidation_counter
    if cls._abc_negative_cache_version != version:
        cls._abc_negative_cache = _ABCWeakSet()
        cls._abc_negative_cache_version = version
    elif candidate in cls._abc_negative_cache:
        return False
    outcome = cls.__subclasshook__(candidate)
    if outcome is not NotImplemented:
        assert isinstance(outcome, bool)
    elif cls in candidate.__mro__:
        outcome = True
    else:
        outcome = any(issubclass(candidate, registered) for registered in cls._abc_registry)
        if not outcome:
            outcome = any(issubclass(candidate, child) for child in cls.__subclasses__())
    if outcome:
        cls._abc_cache.add(candidate)
    else:
        cls._abc_negative_cache.add(candidate)
    return outcome

ABCMeta.__instancecheck__ = _runtime_abc_instancecheck
ABCMeta.__subclasscheck__ = _runtime_abc_subclasscheck

# Register a virtual subclass after checking existing membership and inheritance cycles.
def _runtime_abc_register(cls, candidate):
    if not isinstance(candidate, type):
        raise TypeError('Can only register classes')
    if issubclass(candidate, cls):
        return candidate
    if issubclass(cls, candidate):
        raise RuntimeError('Refusing to create an inheritance cycle')
    cls._abc_registry.add(candidate)
    ABCMeta._abc_invalidation_counter += 1
    return candidate

ABCMeta.register = _runtime_abc_register

# Preserve ABC registration and propagate the native pattern protocol flags.
_abc_register_original = ABCMeta.register

def _abc_register_with_protocol(cls, subclass):
    result = _abc_register_original(cls, subclass)
    flags = getattr(cls, '__abc_tpflags__', 0)
    if flags:
        try:
            subclass.__abc_tpflags__ = flags
        except (AttributeError, TypeError):
            pass
    return result

ABCMeta.register = _abc_register_with_protocol
