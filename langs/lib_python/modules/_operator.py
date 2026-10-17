# Runtime adapter for the CPython 3.14 _operator accelerator.
# Lib/operator.py stays unchanged; native index conversion ignores int subclass overrides.

__doc__ = 'Operator accelerator supplied by the Python runtime.'
__all__ = ['add', 'index']


class _BuiltinOperator:
    """Callable with the non-binding behavior of a C builtin.

    CPython's _operator is a C module, so its functions carry no
    ``__get__``: a plain assignment such as ``concat_path = operator.add``
    stays a function instead of turning into a bound method.  A Python
    ``def`` would bind ``self``, so this adapter hands ``Lib/operator.py``
    instances of this class through its ``from _operator import *``.
    """

    def __init__(self, name, func):
        self.__name__ = name
        self.__qualname__ = name
        self.__doc__ = getattr(func, '__doc__', None)
        self._func = func
        # inspect.signature follows __wrapped__; without it the generic
        # __call__ below would report every operator as (*args, **kwargs).
        self.__wrapped__ = func

    def __call__(self, *args, **kwargs):
        return self._func(*args, **kwargs)

    def __reduce__(self):
        # A C builtin pickles by name and is re-found in its module.  Return
        # the name so ``pickle`` looks this callable up as ``_operator.add``.
        return self.__name__

    def __repr__(self):
        return '<built-in function ' + self.__name__ + '>'


class _Index(_BuiltinOperator):
    def __init__(self):
        self.__name__ = self.__qualname__ = "index"
        self.__doc__ = None

    @property
    def __wrapped__(self):
        return self.__call__

    def __call__(self, a, /):
        if isinstance(a, int):
            return int.__index__(a)
        method = getattr(type(a), '__index__', None)
        if method is not None:
            answer = method(a)
            if isinstance(answer, int):
                if type(answer) is not int:
                    import warnings
                    warnings.warn('__index__ returned non-int (type ' + type(answer).__name__.encode('utf-8')[:200].decode('utf-8', 'ignore') +
                                  ').  The ability to return an instance of a strict subclass of int '
                                  'is deprecated, and may be removed in a future version of Python.',
                                  DeprecationWarning, stacklevel=2)
                return int.__index__(answer)
            raise TypeError('__index__ returned non-int (type ' + type(answer).__name__.encode('utf-8')[:200].decode('utf-8', 'ignore') + ')')
        raise TypeError("'" + type(a).__name__.encode('utf-8')[:200].decode('utf-8', 'ignore') + "' object cannot be interpreted as an integer")


index = _Index()


def _replace_operator_functions():
    """Give Lib/operator.py builtin-style functions instead of descriptors.

    ``operator.py`` runs ``from _operator import *`` near its end, so this
    adapter normally sees a module whose definitions already exist.  When
    this adapter is loaded first, importing operator completes that module
    and the replacements are written into it directly, aliases included.
    """
    import sys
    module = sys.modules.get('operator')
    if module is None:
        import operator as module
    replacements = {}
    for name in list(getattr(module, '__all__', ())):
        if name == 'index':
            continue
        value = getattr(module, name, None)
        if value is None or isinstance(value, type) or not callable(value):
            continue
        replacements[id(value)] = _BuiltinOperator(name, value)
    namespace = globals()
    for wrapper in replacements.values():
        namespace[wrapper.__name__] = wrapper
        __all__.append(wrapper.__name__)
    for name, member in list(module.__dict__.items()):
        wrapper = replacements.get(id(member))
        if wrapper is not None:
            setattr(module, name, wrapper)


_replace_operator_functions()


def _get_add():
    return add


class _AddBridge:
    """Callable bridge with the non-binding behavior of a C builtin."""
    __module__ = '_operator'
    __name__ = 'add'

    def __call__(self, a, b, /):
        return a + b

    def __reduce__(self):
        return (_get_add, ())


add = _AddBridge()


_native_compare = __crypto

class _DigestComparison:
    """Callable bridge with the non-binding behavior of a C builtin."""
    __name__ = '_compare_digest'

    def __call__(self, a, b, /):
        if isinstance(a, str) and isinstance(b, str):
            a = str.__getitem__(a, slice(None))
            b = str.__getitem__(b, slice(None))
            if not a.isascii() or not b.isascii():
                raise TypeError('comparing strings with non-ASCII characters is not supported')
            a, b = a.encode('ascii'), b.encode('ascii')
        else:
            import array
            buffers = (bytes, bytearray, memoryview, array.array)
            if not isinstance(a, buffers) and not isinstance(b, buffers):
                raise TypeError("unsupported operand types(s) or combination of types: '" + type(a).__name__[:100] + "' and '" + type(b).__name__[:100] + "'")
            converted = []
            for operand in (a, b):
                if not isinstance(operand, buffers):
                    raise TypeError("a bytes-like object is required, not '" + type(operand).__name__ + "'")
                if isinstance(operand, (bytes, bytearray)):
                    converted.append(bytes(operand))
                    continue
                view = memoryview(operand)
                view._check()
                if not view.c_contiguous:
                    raise BufferError('memoryview: underlying buffer is not C-contiguous')
                converted.append(view.tobytes())
            a, b = converted
        return _native_compare(2, a, b)

_compare_digest = _DigestComparison()


class _Addition:
    """Callable counterpart of the non-binding _operator.add builtin."""
    __name__ = 'add'
    __qualname__ = 'add'
    __module__ = '_operator'
    __doc__ = 'Same as a + b.'

    # Expose the positional-only signature while preserving native call validation.
    @property
    def __wrapped__(self):
        return _AddBridge().__call__

    # Apply Python addition after checking the builtin calling convention.
    def __call__(self, *args, **kwargs):
        if kwargs:
            raise TypeError('_operator.add() takes no keyword arguments')
        if len(args) != 2:
            raise TypeError('add expected 2 arguments, got ' + str(len(args)))
        return args[0] + args[1]

    # Native functions pickle by their module-global name in every protocol.
    def __reduce__(self):
        return 'add'

    def __reduce_ex__(self, protocol):
        return 'add'

add = _Addition()


# A fresh accelerator can import operator recursively before these exports exist.
# Complete the public aliases after the native-style addition has been created.
import sys as _sys
_operator_module = _sys.modules.get('operator')
if _operator_module is not None:
    _operator_module.add = add
    _operator_module.__add__ = add
