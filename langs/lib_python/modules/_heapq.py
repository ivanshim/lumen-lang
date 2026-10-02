# Runtime adapter derived from CPython Lib/heapq.py at v3.14.8 / 8e6e75d9102e; PSF License.
# Python entry points for the native list-only heap accelerator.
__all__ = ['heapify', 'heappop', 'heappush', 'heappushpop', 'heapreplace', 'heapify_max', 'heappop_max', 'heappush_max', 'heappushpop_max', 'heapreplace_max']

def heappush(heap, item, /):
    """Push item onto heap, maintaining the heap invariant."""
    return __heap_native__(heap, 'heappush', item)

def heappop(heap, /):
    """Pop the smallest item off the heap, maintaining the heap invariant."""
    return __heap_native__(heap, 'heappop')

def heapreplace(heap, item, /):
    """Pop and return the current smallest value, and add the new item.

    This is more efficient than heappop() followed by heappush(), and can be
    more appropriate when using a fixed-size heap.  Note that the value
    returned may be larger than item!  That constrains reasonable uses of
    this routine unless written as part of a conditional replacement:

        if item > heap[0]:
            item = heapreplace(heap, item)
    """
    return __heap_native__(heap, 'heapreplace', item)

def heappushpop(heap, item, /):
    """Fast version of a heappush followed by a heappop."""
    return __heap_native__(heap, 'heappushpop', item)

def heapify(x, /):
    """Transform list into a heap, in-place, in O(len(x)) time."""
    return __heap_native__(x, 'heapify')

def heappop_max(heap, /):
    """Maxheap version of a heappop."""
    return __heap_native__(heap, 'heappop_max')

def heapreplace_max(heap, item, /):
    """Maxheap version of a heappop followed by a heappush."""
    return __heap_native__(heap, 'heapreplace_max', item)

def heappush_max(heap, item, /):
    """Maxheap version of a heappush."""
    return __heap_native__(heap, 'heappush_max', item)

def heappushpop_max(heap, item, /):
    """Maxheap fast version of a heappush followed by a heappop."""
    return __heap_native__(heap, 'heappushpop_max', item)

def heapify_max(x, /):
    """Transform list into a maxheap, in-place, in O(len(x)) time."""
    return __heap_native__(x, 'heapify_max')
