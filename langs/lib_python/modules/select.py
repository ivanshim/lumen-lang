# Native select adapter; CPython v3.14.8 Modules/selectmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
import math as _math
from posix import _call as _posix_call

error = OSError

FD_SETSIZE = 1024


def _as_fd(item):
    # PyObject_AsFileDescriptor: an int (a bool first warns), or whatever
    # the object's own fileno attribute says when called with no argument.
    if isinstance(item, int):
        if isinstance(item, bool):
            import warnings
            warnings.warn('bool is used as a file descriptor', RuntimeWarning, stacklevel=2)
        fd = item
    else:
        meth = getattr(item, 'fileno', None)
        if meth is None:
            raise TypeError('argument must be an int, or have a fileno() method.')
        fd = meth()
        if not isinstance(fd, int):
            raise TypeError('fileno() returned a non-integer')
    if fd > 2147483647 or fd < -2147483648:
        raise OverflowError('Python int too large to convert to C int')
    if fd < 0:
        raise ValueError('file descriptor cannot be a negative integer (%d)' % (fd,))
    if fd >= FD_SETSIZE:
        raise ValueError('filedescriptor out of range in select()')
    return int(fd)


class _Select:
    # A callable object, not a function: the reference's select is a C
    # builtin, and builtins stored as class attributes (selectors.py keeps
    # `_select = select.select` on SelectSelector) do not bind as methods,
    # so neither may this one.
    def __call__(self, rlist, wlist, xlist, timeout=None):
        if timeout is None:
            usec = -1
        elif isinstance(timeout, (int, float)):
            if timeout < 0:
                raise ValueError('timeout must be non-negative')
            # _PyTime_FromSecondsObject with _PyTime_ROUND_TIMEOUT, which is
            # ROUND_UP: a positive sub-microsecond timeout still waits.
            usec = (_math.ceil(timeout * 1_000_000_000) + 999) // 1000
        else:
            raise TypeError('timeout must be a float or None')
        waiting = []
        for items in (rlist, wlist, xlist):
            items = list(items)
            waiting.append((items, [_as_fd(item) for item in items]))
        ready = _posix_call('select', waiting[0][1], waiting[1][1], waiting[2][1], usec)
        return tuple([item for item, fd in zip(items, fds) if fd in ready[at]]
                     for at, (items, fds) in enumerate(waiting))


select = _Select()
