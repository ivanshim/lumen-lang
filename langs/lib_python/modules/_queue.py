# Unbounded FIFO storage with runtime method descriptors.
from collections import deque
_method_descriptor = __method_descriptor
import math
import time
import types

class Empty(Exception):
    """Exception raised when a non-blocking or timed get finds no item."""

class SimpleQueue:
    """Simple, unbounded FIFO queue."""
    __slots__ = ('_items', '__weakref__')

    def __init__(self):
        self._items = deque()

    def put(self, item, block=True, timeout=None):
        self._items.append(item)

    def put_nowait(self, item):
        self._items.append(item)

    def get(self, block=True, timeout=None):
        if block and timeout is not None:
            if not isinstance(timeout, (int, float)):
                raise TypeError("'" + type(timeout).__name__ + "' object cannot be interpreted as an integer or float")
            timeout = float(timeout)
            if math.isnan(timeout):
                raise ValueError('Invalid value NaN (not a number)')
            if timeout < 0:
                raise ValueError("'timeout' must be a non-negative number")
            if timeout > 9223372036.0:
                raise OverflowError('timestamp out of range for C PyTime_t')
        if self._items:
            return self._items.popleft()
        if not block:
            raise Empty
        if timeout is None:
            # shortcut: the runtime cannot start a producer thread; upgrade when threads are supported.
            raise RuntimeError('no other thread can put an item on this queue')
        deadline = time.monotonic() + timeout
        while not self._items:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise Empty
            time.sleep(min(remaining, 0.01))
        return self._items.popleft()

    def get_nowait(self):
        if not self._items:
            raise Empty
        return self._items.popleft()

    def empty(self):
        return not self._items

    def qsize(self):
        return len(self._items)

    __class_getitem__ = classmethod(types.GenericAlias)

for _name in ('put', 'put_nowait', 'get', 'get_nowait', 'empty', 'qsize'):
    setattr(SimpleQueue, _name, _method_descriptor(SimpleQueue, getattr(SimpleQueue, _name)))
del _name, _method_descriptor
