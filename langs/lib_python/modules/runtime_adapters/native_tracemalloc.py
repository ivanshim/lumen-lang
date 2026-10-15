# Runtime allocation tracing adapter; Frame and Traceback definitions from CPython
# v3.14.8 / 8e6e75d9102e, Lib/tracemalloc.py; PSF License.
import os
import linecache
from collections.abc import Sequence
from functools import total_ordering

_native_trace = __allocation_trace

def is_tracing():
    return _native_trace(3)

def start(nframe=1):
    from operator import index
    nframe = index(nframe)
    if not 1 <= nframe <= 65535:
        raise ValueError('the number of frames must be in range [1; 65535]')
    return _native_trace(1, int(nframe))

def stop():
    return _native_trace(2)

def get_traceback_limit():
    return _native_trace(4)

def _get_object_traceback(obj):
    frames = _native_trace(0, obj)
    if frames is None:
        return None
    return tuple(reversed(frames))

@total_ordering
class Frame:
    """
    Frame of a traceback.
    """
    __slots__ = ("_frame",)

    def __init__(self, frame):
        # frame is a tuple: (filename: str, lineno: int)
        self._frame = frame

    @property
    def filename(self):
        return self._frame[0]

    @property
    def lineno(self):
        return self._frame[1]

    def __eq__(self, other):
        if not isinstance(other, Frame):
            return NotImplemented
        return (self._frame == other._frame)

    def __lt__(self, other):
        if not isinstance(other, Frame):
            return NotImplemented
        return (self._frame < other._frame)

    def __hash__(self):
        return hash(self._frame)

    def __str__(self):
        return "%s:%s" % (self.filename, self.lineno)

    def __repr__(self):
        return "<Frame filename=%r lineno=%r>" % (self.filename, self.lineno)


@total_ordering
class Traceback(Sequence):
    """
    Sequence of Frame instances sorted from the oldest frame
    to the most recent frame.
    """
    __slots__ = ("_frames", '_total_nframe')

    def __init__(self, frames, total_nframe=None):
        Sequence.__init__(self)
        # frames is a tuple of frame tuples: see Frame constructor for the
        # format of a frame tuple; it is reversed, because _tracemalloc
        # returns frames sorted from most recent to oldest, but the
        # Python API expects oldest to most recent
        self._frames = tuple(reversed(frames))
        self._total_nframe = total_nframe

    @property
    def total_nframe(self):
        return self._total_nframe

    def __len__(self):
        return len(self._frames)

    def __getitem__(self, index):
        if isinstance(index, slice):
            return tuple(Frame(trace) for trace in self._frames[index])
        else:
            return Frame(self._frames[index])

    def __contains__(self, frame):
        return frame._frame in self._frames

    def __hash__(self):
        return hash(self._frames)

    def __eq__(self, other):
        if not isinstance(other, Traceback):
            return NotImplemented
        return (self._frames == other._frames)

    def __lt__(self, other):
        if not isinstance(other, Traceback):
            return NotImplemented
        return (self._frames < other._frames)

    def __str__(self):
        return str(self[0])

    def __repr__(self):
        s = f"<Traceback {tuple(self)}"
        if self._total_nframe is None:
            s += ">"
        else:
            s += f" total_nframe={self.total_nframe}>"
        return s

    def format(self, limit=None, most_recent_first=False):
        lines = []
        if limit is not None:
            if limit > 0:
                frame_slice = self[-limit:]
            else:
                frame_slice = self[:limit]
        else:
            frame_slice = self

        if most_recent_first:
            frame_slice = reversed(frame_slice)
        for frame in frame_slice:
            lines.append('  File "%s", line %s'
                         % (frame.filename, frame.lineno))
            line = linecache.getline(frame.filename, frame.lineno).strip()
            if line:
                lines.append('    %s' % line)
        return lines


def get_object_traceback(obj):
    """
    Get the traceback where the Python object *obj* was allocated.
    Return a Traceback instance.

    Return None if the tracemalloc module is not tracing memory allocations or
    did not trace the allocation of the object.
    """
    frames = _get_object_traceback(obj)
    if frames is not None:
        return Traceback(frames)
    else:
        return None


