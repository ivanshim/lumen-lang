# Descriptor readiness through the native POSIX poll interface.
# CPython v3.14.8 Modules/selectmodule.c semantics; PSF License.
import math
import operator
from posix import _call
error = OSError


def _fileno(object):
    try:
        fd = operator.index(object)
    except TypeError:
        fd = operator.index(object.fileno())
    if fd < 0:
        raise ValueError('file descriptor cannot be a negative integer')
    return fd


def _select(rlist, wlist, xlist, timeout=None, /):
    if timeout is None:
        milliseconds = -1
    else:
        timeout = float(timeout)
        if timeout < 0:
            raise ValueError('timeout must be non-negative')
        milliseconds = math.ceil(timeout * 1000)
    readers = [(object, _fileno(object)) for object in rlist]
    writers = [(object, _fileno(object)) for object in wlist]
    exceptional = [(object, _fileno(object)) for object in xlist]
    requests = {}
    for group, mask in ((readers, 1), (writers, 4), (exceptional, 2)):
        for object, fd in group:
            requests[fd] = requests.get(fd, 0) | mask
    events = dict(_call('poll', tuple(requests.items()), milliseconds))
    if any(mask & 32 for mask in events.values()):
        raise OSError(9, 'Bad file descriptor')
    return ([object for object, fd in readers if events.get(fd, 0) & (1 | 8 | 16)],
            [object for object, fd in writers if events.get(fd, 0) & (4 | 8 | 16)],
            [object for object, fd in exceptional if events.get(fd, 0) & 2])


class _SelectFunction:
    # A C module callable does not bind when stored on a class.
    def __call__(self, rlist, wlist, xlist, timeout=None, /):
        return _select(rlist, wlist, xlist, timeout)


select = _SelectFunction()
