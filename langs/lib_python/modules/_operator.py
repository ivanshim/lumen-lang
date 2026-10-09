# Runtime adapter for the CPython 3.14 _operator accelerator.
# Lib/operator.py stays unchanged; native index conversion ignores int subclass overrides.

__doc__ = 'Operator accelerator supplied by the Python runtime.'
__all__ = ['index', 'add']

def index(a):
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
    """Provide the non-binding callable supplied by the operator accelerator."""
    __name__ = 'add'
    __qualname__ = 'add'
    __module__ = '_operator'

    # Add operands through their ordinary Python arithmetic dispatch.
    def __call__(self, a, b, /):
        return a + b

    def __reduce__(self):
        return self.__name__

    def __reduce_ex__(self, protocol):
        return self.__reduce__()


add = _Addition()
