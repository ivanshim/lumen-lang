# Buffer slot wrappers model CPython 3b564385e4c9, Objects/typeobject.c (PSF License).
from operator import index
import sys


def getbuffer(owner, flags):
    flags = index(flags)
    if flags > sys.maxsize or flags < -sys.maxsize - 1:
        raise OverflowError('Python int too large to convert to C ssize_t')
    if flags > 2147483647 or flags < -2147483648:
        raise OverflowError('buffer flags out of range')
    view = memoryview(owner)
    if flags & 1 and view.readonly:
        view.release()
        raise BufferError('Object is not writable.')
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
