# Runtime adapter for the CPython 3.14 _operator accelerator.
# Lib/operator.py stays unchanged; native index conversion ignores int subclass overrides.

__doc__ = 'Operator accelerator supplied by the Python runtime.'
__all__ = ['index']

def index(a):
    if isinstance(a, int):
        return int.__index__(a)
    method = getattr(type(a), '__index__', None)
    if method is not None:
        answer = method(a)
        if isinstance(answer, int):
            if type(answer) is not int:
                import warnings
                warnings.warn('__index__ returned non-int (type ' + type(answer).__name__ +
                              ').  The ability to return an instance of a strict subclass of int '
                              'is deprecated, and may be removed in a future version of Python.',
                              DeprecationWarning, stacklevel=2)
            return int.__index__(answer)
        raise 'TypeError: __index__ returned non-int (type ' + type(answer).__name__ + ')'
    raise 'TypeError: value cannot be interpreted as an integer'


