# The accelerated ABC instance operation performs the underlying ABC
# subclass search before typing checks instance attributes. A protocol's
# public subclass operation rejects data members; that restriction belongs
# to issubclass, rather than the internal ABC search during isinstance.
_original_abc_instancecheck = ABCMeta.__instancecheck__

def _abc_instancecheck(cls, instance):
    if getattr(cls, '_is_protocol', False) and getattr(cls, '_is_runtime_protocol', False):
        actual = type(instance)
        reported = getattr(instance, '__class__', actual)
        return ABCMeta.__subclasscheck__(cls, reported) or (
            actual is not reported and ABCMeta.__subclasscheck__(cls, actual))
    return _original_abc_instancecheck(cls, instance)

ABCMeta.__instancecheck__ = _abc_instancecheck

_original_abc_register = ABCMeta.register

def _abc_register(cls, subclass):
    if not getattr(cls, '_is_protocol', False):
        return _original_abc_register(cls, subclass)
    if not isinstance(subclass, type):
        raise TypeError('Can only register classes')
    if ABCMeta.__subclasscheck__(cls, subclass):
        return subclass
    if issubclass(cls, subclass):
        raise RuntimeError('Refusing to create an inheritance cycle')
    cls._abc_registry.add(subclass)
    ABCMeta._abc_invalidation_counter += 1
    return subclass

ABCMeta.register = _abc_register
