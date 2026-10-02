# Object reduction support corresponding to CPython Objects/typeobject.c.
# PSF License.

def state(value):
    from pickle import _reduction_hook
    getter = _reduction_hook(value, '__getstate__')
    if getter is not None:
        return getter()
    namespace = getattr(value, '__dict__', None)
    if namespace is not None and not namespace:
        namespace = None
    slots = {}
    for cls in type(value).__mro__:
        names = getattr(cls, '__slots__', ())
        if isinstance(names, str):
            names = (names,)
        for name in names:
            if name in ('__dict__', '__weakref__'):
                continue
            if name.startswith('__') and not name.endswith('__'):
                name = '_' + cls.__name__.lstrip('_') + name
            if hasattr(value, name):
                slots[name] = getattr(value, name)
    return (namespace, slots) if slots else namespace

def reduce(value, protocol=0):
    import copyreg
    from operator import index
    protocol = index(protocol)
    if not -2147483648 <= protocol <= 2147483647:
        raise OverflowError('Python int too large to convert to C int')
    if protocol < 2:
        return copyreg._reduce_ex(value, protocol)
    cls = type(value)
    new_ex = getattr(value, '__getnewargs_ex__', None)
    if new_ex is not None:
        supplied = new_ex()
        if not isinstance(supplied, tuple) or len(supplied) != 2:
            raise TypeError('__getnewargs_ex__ should return a tuple of length 2')
        args, kwargs = supplied
        if not isinstance(args, tuple) or not isinstance(kwargs, dict):
            raise TypeError('__getnewargs_ex__ should return a tuple and a dict')
        maker = copyreg.__newobj_ex__
        arguments = (cls, args, kwargs)
    else:
        new_args = getattr(value, '__getnewargs__', None)
        if new_args is None and isinstance(value, tuple):
            args = (tuple(value),)
        else:
            args = () if new_args is None else new_args()
        if not isinstance(args, tuple):
            raise TypeError('__getnewargs__ should return a tuple')
        maker = copyreg.__newobj__
        arguments = (cls, *args)
    items = iter(value) if isinstance(value, list) else None
    pairs = iter(value.items()) if isinstance(value, dict) else None
    return maker, arguments, state(value), items, pairs
