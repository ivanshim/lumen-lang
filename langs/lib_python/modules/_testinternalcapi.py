# What the reference implementation's own internals answer about its
# objects. Only the probes this runtime can answer truthfully stand
# here; the rest refuse at the bottom of the file.

import signal as _signal

def has_inline_values(obj):
    # The kernel tracks the compact layout until dictionary replacement,
    # deletion, or growth beyond its available attribute places.
    return _has_inline_values(obj)


class SelfInterruptingContextManager:
    # The reference's own is written in C, where no evaluation check
    # stands between its words: it leaves an interrupt pending as it
    # enters, so the KeyboardInterrupt lands on the first statement of
    # the body and the leaving method runs all the same. The same is
    # arranged here by shape: the raise is the entering method's last
    # statement, so no statement's edge stands between the pending note
    # and the frame's end, and the interrupt is taken up as the body
    # begins.
    def __init__(self):
        self._within = False

    def __enter__(self):
        self._within = True
        _signal.raise_signal(_signal.SIGINT)

    def __exit__(self, *args):
        self._within = False
        return False

    def within(self):
        return self._within


# The reference's own module reports this many runs before a
# specialization is made; the same number stands here so the runs the
# tests make of it run as they run there.
SPECIALIZATION_THRESHOLD = 2


def get_recursion_depth():
    import sys
    frame = sys._getframe(1)
    depth = 0
    while frame is not None:
        depth += 1
        frame = frame.f_back
    return depth


def __getattr__(name):
    raise 'NotImplementedError: _testinternalcapi.' + name + ' is not supported'
