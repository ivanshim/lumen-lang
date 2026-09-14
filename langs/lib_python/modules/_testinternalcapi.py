# What the reference implementation's own internals answer about its
# objects. Only the probes this runtime can answer truthfully stand
# here; the rest refuse at the bottom of the file.

def has_inline_values(obj):
    # CPython asks whether a thing keeps its attributes in an array laid
    # out inside the object itself, with the dictionary made only when
    # something asks for one. No object of this runtime does: an
    # instance keeps its attributes in the very dictionary `__dict__`
    # hands out, and writing into that dictionary is writing the
    # attribute. So the true answer here is no, for anything, and a test
    # that looks for the inline layout fails because the layout is not
    # here rather than because the probe is missing.
    return False


class SelfInterruptingContextManager:
    # CPython's own leaves an interrupt pending as it enters, so the
    # KeyboardInterrupt lands on the first statement of the body and a
    # test can see that __exit__ ran all the same. Nothing here can make
    # a signal arrive between two statements. Raising the
    # KeyboardInterrupt from __enter__ instead would skip __exit__
    # altogether and leave the test reading as passed, so the thing
    # refuses to be made at all and says what is missing.
    def __init__(self):
        raise 'NotImplementedError: SelfInterruptingContextManager needs an interrupt delivered between two statements of the body, which this runtime cannot arrange'


def __getattr__(name):
    raise 'NotImplementedError: _testinternalcapi.' + name + ' is not supported'
