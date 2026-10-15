# Async iteration uses type-level special methods, like the other protocols.
_async_next_missing = object()

class _AsyncNextAwaitable:
    def __init__(self, awaitable, default):
        self._awaitable = awaitable
        self._default = default
        self._iterator = None

    def __await__(self):
        return self

    def __iter__(self):
        return self

    def __next__(self):
        return self.send(None)

    async def _result(self):
        try:
            return await self._awaitable
        except StopAsyncIteration:
            return self._default

    def _walk(self):
        if self._iterator is None:
            self._iterator = self._result().__await__()
        return self._iterator

    def send(self, value):
        return self._walk().send(value)

    def throw(self, *args):
        return self._walk().throw(*args)

    def close(self):
        if self._iterator is not None:
            return self._iterator.close()
        close = getattr(self._awaitable, 'close', None)
        if close is not None:
            return close()


def _async_special(subject, name):
    cls = type(subject)
    for base in cls.__mro__:
        namespace = vars(base)
        if name in namespace:
            descriptor = namespace[name]
            bind = getattr(type(descriptor), '__get__', None)
            return bind(descriptor, subject, cls) if bind is not None else descriptor
    return None


def anext(iterator, default=_async_next_missing, /):
    method = _async_special(iterator, '__anext__')
    if method is None:
        raise TypeError("'" + type(iterator).__name__ + "' object is not an async iterator")
    awaitable = method()
    if default is _async_next_missing:
        return awaitable
    return _AsyncNextAwaitable(awaitable, default)


def aiter(iterable, /):
    method = _async_special(iterable, '__aiter__')
    if method is None:
        raise TypeError("'" + type(iterable).__name__ + "' object is not an async iterable")
    iterator = method()
    if getattr(type(iterator), '__anext__', None) is None:
        raise TypeError("aiter() returned not an async iterator of type '" + type(iterator).__name__ + "'")
    return iterator
