# Runtime binding for CPython v3.14.8 Objects/picklebufobject.c.
class PickleBuffer:
    __slots__ = ('_handle', '__weakref__')
    __module__ = 'pickle'
    def __new__(cls, *args, **kwargs):
        count = len(args) + len(kwargs)
        if count > 1:
            category = 'keyword argument' if not args else 'argument'
            raise TypeError('PickleBuffer() takes at most 1 ' + category + ' (' + str(count) + ' given)')
        if not args:
            raise TypeError('PickleBuffer() takes exactly 1 positional argument (0 given)')
        return __pickle_buffer_native__(4, cls, args[0])
    def raw(self):
        return __pickle_buffer_native__(1, self._handle, self)
    def release(self):
        return __pickle_buffer_native__(2, self._handle)
    def __buffer__(self, flags, /):
        return __pickle_buffer_native__(3, self._handle, self, flags)
    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'pickle.PickleBuffer' is not an acceptable base type")

__pickle_buffer_native__(5, PickleBuffer)
