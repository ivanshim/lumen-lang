# Unbounded FIFO storage with runtime method descriptors.
from collections import deque
_method_descriptor = __method_descriptor
import math
import operator
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
        block = bool(block)
        if block and timeout is not None:
            bound = 2**63
            if isinstance(timeout, float):
                timeout = float.__float__(timeout)
                if math.isnan(timeout):
                    raise ValueError('Invalid value NaN (not a number)')
                if timeout < -bound / 10**9 or timeout >= bound / 10**9:
                    raise OverflowError('timestamp out of range for C PyTime_t')
            else:
                try:
                    timeout = operator.index(timeout)
                except TypeError:
                    raise TypeError("'" + type(timeout).__name__ + "' object cannot be interpreted as an integer or float") from None
                except OverflowError:
                    raise OverflowError('timestamp out of range for C PyTime_t') from None
                nanoseconds = timeout * 10**9
                if nanoseconds < -bound or nanoseconds >= bound:
                    raise OverflowError('timestamp out of range for C PyTime_t')
                timeout = float(timeout)
            if timeout < 0:
                raise ValueError("'timeout' must be a non-negative number")
        if self._items:
            return self._items.popleft()
        if not block:
            raise Empty
        deadline = None if timeout is None else time.monotonic() + timeout
        # shortcut: waits poll; use a condition when shared interpreter threads are supported.
        while not self._items:
            delay = 0.01
            if deadline is not None:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise Empty
                delay = min(remaining, delay)
            time.sleep(delay)
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
