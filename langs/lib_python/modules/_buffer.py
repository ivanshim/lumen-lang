# Buffer slot wrappers model CPython v3.14.8, Objects/typeobject.c (PSF License).
from operator import index
import sys


def getbuffer(owner, flags):
    if not isinstance(flags, int) and not hasattr(type(flags), "__index__"):
        raise TypeError("'" + type(flags).__name__ + "' object cannot be interpreted as an integer")
    flags = index(flags)
    if flags > sys.maxsize or flags < -sys.maxsize - 1:
        raise OverflowError("cannot fit 'int' into an index-sized integer")
    if flags > 2147483647 or flags < -2147483648:
        raise OverflowError('buffer flags out of range')
    view = memoryview(owner)
    if flags & 1 and view.readonly:
        view.release()
        message = 'memoryview: underlying buffer is not writable' if isinstance(owner, memoryview) else 'Object is not writable.'
        raise BufferError(message)
    if isinstance(owner, memoryview):
        # Respect the exporter's layout before weakening the requested metadata.
        error = None
        if flags & 32 and not view.c_contiguous:
            error = 'memoryview: underlying buffer is not C-contiguous'
        elif flags & 64 and not view.f_contiguous:
            error = 'memoryview: underlying buffer is not Fortran contiguous'
        elif flags & 128 and not view.contiguous:
            error = 'memoryview: underlying buffer is not contiguous'
        elif not flags & 16 and not view.c_contiguous:
            error = 'memoryview: underlying buffer is not C-contiguous'
        elif not flags & 8 and flags & 4:
            error = 'memoryview: cannot cast to unsigned bytes if the format flag is present'
        if error is not None:
            view.release()
            raise BufferError(error)
        if not flags & 4:
            view._format = 'B'
        if not flags & 8:
            view._shape = (view.nbytes // view.itemsize,)
        if not flags & 16 and len(view._offsets) <= 1:
            start = view._offsets.start if isinstance(view._offsets, range) else view._offsets[0] if view._offsets else 0
            view._offsets = range(start, start + len(view._offsets) * view.itemsize, view.itemsize)
    view._buffer_owner = owner
    return view


def releasebuffer(owner, view):
    if not isinstance(view, memoryview):
        raise TypeError('expected a memoryview object')
    if view._released:
        return None
    if getattr(view, '_buffer_owner', view._source) is not owner:
        raise ValueError("memoryview's buffer is not this object")
    view.release()
