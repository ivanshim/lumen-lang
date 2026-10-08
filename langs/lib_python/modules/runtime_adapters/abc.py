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

# Keep instance-check frames in abc, matching the public metaclass module.
def _abc_instancecheck_in_module(cls, instance):
    """Override for isinstance(instance, cls)."""
    # Inline the cache checking
    try:
        subclass = instance.__class__
    except AttributeError:
        # Fall back to the type when the instance has no __class__,
        # matching the behaviour of the built-in isinstance() (gh-153772).
        subclass = type(instance)
    if subclass in cls._abc_cache:
        return True
    subtype = type(instance)
    if subtype is subclass:
        if (cls._abc_negative_cache_version ==
            ABCMeta._abc_invalidation_counter and
            subclass in cls._abc_negative_cache):
            return False
        # Fall back to the subclass check.
        return cls.__subclasscheck__(subclass)
    return any(cls.__subclasscheck__(c) for c in (subclass, subtype))

ABCMeta.__instancecheck__ = _abc_instancecheck_in_module
