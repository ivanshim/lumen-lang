"""Collection of the reference rounds that counting alone cannot free.

Values are kept by reference counts, so most go the moment their last
holder does. collect() finds the rounds -- values holding one another
that nothing else reaches -- and breaks them; their objects say their
last words first and any weak reference to them is cleared.
"""
_enabled = True
_threshold = (2000, 10, 0)
garbage = []
callbacks = []

def collect(generation=2):
    return __gc_collect()

def disable():
    global _enabled
    _enabled = False

def enable():
    global _enabled
    _enabled = True

def isenabled():
    return _enabled

def get_threshold():
    return _threshold

def set_threshold(threshold0, threshold1=None, threshold2=None):
    global _threshold
    t = list(_threshold)
    t[0] = threshold0
    if threshold1 is not None:
        t[1] = threshold1
    if threshold2 is not None:
        t[2] = threshold2
    _threshold = tuple(t)

def get_count():
    return (0, 0, 0)

def get_debug():
    return 0

def set_debug(flags):
    pass

def is_tracked(obj):
    return not isinstance(obj, (int, float, str, bytes, bool, type(None)))

def is_finalized(obj):
    return False

def freeze():
    pass

def unfreeze():
    pass

def get_freeze_count():
    return 0
