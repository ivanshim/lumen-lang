# Runtime bridge for CPython v3.14.8 Modules/_abc.c; PSF License.
# Preserve the source-backed registry algorithm and the native call boundary.
from _py_abc import ABCMeta as _RegistryMeta, get_cache_token
from _weakrefset import WeakSet
from _weakref import ref

_abc_instancecheck = __abc_instancecheck

# Initialize abstract members and independent weak registries for a new ABC.
def _abc_init(cls, /):
    abstracts = {name for name, value in cls.__dict__.items()
                 if getattr(value, '__isabstractmethod__', False)}
    for base in cls.__bases__:
        for name in getattr(base, '__abstractmethods__', ()):
            if getattr(getattr(cls, name, None), '__isabstractmethod__', False):
                abstracts.add(name)
    cls.__abstractmethods__ = frozenset(abstracts)
    cls._abc_registry = WeakSet()
    cls._abc_cache = WeakSet()
    cls._abc_negative_cache = WeakSet()
    cls._abc_negative_cache_version = get_cache_token()

# Register virtual subclasses using the standard registry and cycle checks.
def _abc_register(cls, subclass, /):
    return _RegistryMeta.register(cls, subclass)

# Check class membership with the preserved Python registry algorithm.
def _abc_subclasscheck(cls, subclass, /):
    return _RegistryMeta.__subclasscheck__(cls, subclass)

# Resolve cached instance membership before the native bridge calls type hooks.
def _prepare_instancecheck(cls, instance, /):
    try:
        subclass = instance.__class__
    except AttributeError:
        subclass = type(instance)
    if subclass in cls._abc_cache:
        return True, ()
    subtype = type(instance)
    if subtype is subclass:
        if (cls._abc_negative_cache_version == get_cache_token()
                and subclass in cls._abc_negative_cache):
            return False, ()
        return None, (subclass,)
    return None, (subclass, subtype)

# Return weak references and the negative-cache version for ABC diagnostics.
def _get_dump(cls, /):
    return (set(ref(item) for item in cls._abc_registry),
            set(ref(item) for item in cls._abc_cache),
            set(ref(item) for item in cls._abc_negative_cache),
            cls._abc_negative_cache_version)

# Clear virtual subclasses without changing the other ABC caches.
def _reset_registry(cls, /):
    cls._abc_registry.clear()

# Clear positive and negative membership caches.
def _reset_caches(cls, /):
    cls._abc_cache.clear()
    cls._abc_negative_cache.clear()
