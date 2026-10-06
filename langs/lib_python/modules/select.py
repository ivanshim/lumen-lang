# Native select adapter; CPython v3.14.8 Modules/selectmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
from posix import _call as _posix_call

error = OSError

FD_SETSIZE = 1024


def _as_fd(item):
    if isinstance(item, int):
        fd = item
    else:
        fileno = getattr(type(item), 'fileno', None)
        if fileno is None:
            raise TypeError('argument must be an int, or have a fileno() method.')
        fd = fileno(item)
        if not isinstance(fd, int):
            raise TypeError('fileno() returned a non-integer')
    if fd < 0:
        raise ValueError('file descriptor cannot be a negative integer (%d)' % (fd,))
    if fd >= FD_SETSIZE:
        raise ValueError('filedescriptor out of range in select()')
    return fd


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
            usec = int(timeout * 1000000)
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
