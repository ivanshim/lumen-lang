# Native select adapter; CPython v3.14.8 Modules/selectmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
import math as _math
from _operator import index as _index
from posix import _call as _posix_call

error = OSError

FD_SETSIZE = 1024
_missing = object()


def _as_fd(item):
    # PyObject_AsFileDescriptor: an int (a bool first warns), or whatever
    # the object's own fileno attribute says when called with no argument.
    if isinstance(item, int):
        if isinstance(item, bool):
            import warnings
            warnings.warn('bool is used as a file descriptor', RuntimeWarning, stacklevel=2)
        fd = item
    else:
        meth = getattr(item, 'fileno', _missing)
        if meth is _missing:
            raise TypeError('argument must be an int, or have a fileno() method.')
        fd = meth()
        if not isinstance(fd, int):
            raise TypeError('fileno() returned a non-integer')
    # PyLong_AsInt reads the stored integer, bypassing subclass hooks.
    fd = int.__index__(fd)
    if fd > 2147483647 or fd < -2147483648:
        raise OverflowError('Python int too large to convert to C int')
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
    def __call__(self, /, *args, **kwargs):
        # The C entry point rejects keywords before checking positional
        # arity. Keep that order and its errors on this callable bridge.
        if kwargs:
            raise TypeError('select.select() takes no keyword arguments')
        count = len(args)
        if count < 3:
            raise TypeError('select expected at least 3 arguments, got %d' % count)
        if count > 4:
            raise TypeError('select expected at most 4 arguments, got %d' % count)
        rlist, wlist, xlist = args[:3]
        timeout = args[3] if count == 4 else None
        if timeout is None:
            usec = -1
        else:
            # PyTime uses signed 64-bit nanoseconds, with rounding away
            # from zero before range checks; non-floats use __index__.
            if isinstance(timeout, float):
                seconds = float.__float__(timeout)
                if _math.isnan(seconds):
                    raise ValueError('Invalid value NaN (not a number)')
                scaled = seconds * 1_000_000_000
                if not -9223372036854775808 <= scaled < 9223372036854775808:
                    raise OverflowError('timestamp out of range for C PyTime_t')
                ns = _math.ceil(scaled) if scaled >= 0 else _math.floor(scaled)
            else:
                try:
                    ns = _index(timeout) * 1_000_000_000
                except TypeError:
                    raise TypeError('timeout must be a float or None')
                except OverflowError:
                    raise OverflowError('timestamp out of range for C PyTime_t')
                if not -9223372036854775808 <= ns <= 9223372036854775807:
                    raise OverflowError('timestamp out of range for C PyTime_t')
            if ns < 0:
                raise ValueError('timeout must be non-negative')
            usec = (ns + 999) // 1000
        waiting = []
        for items in (rlist, wlist, xlist):
            items = list(items)
            fds = []
            for item in items:
                fd = _as_fd(item)
                # Conversion of the overflowing entry precedes the count
                # check, just as seq2set does, including duplicate entries.
                if len(fds) >= FD_SETSIZE:
                    raise ValueError('too many file descriptors in select()')
                fds.append(fd)
            waiting.append((items, fds))
        ready = _posix_call('select', waiting[0][1], waiting[1][1], waiting[2][1], usec)
        return tuple([item for item, fd in zip(items, fds) if fd in ready[at]]
                     for at, (items, fds) in enumerate(waiting))


select = _Select()
