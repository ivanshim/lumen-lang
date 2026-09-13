# What a program can find out about the things it is made of.
#
# Most of CPython's inspect reads a code object: the flags a function
# was compiled with, the names of its arguments, the line it starts on,
# the frame a generator is stopped in. The reader here hands out no such
# thing. A function answers to __code__ with a wrapper that carries none
# of those, a generator keeps no frame a program can reach, and there is
# no list of the calls in progress. So the part of this module that
# reads a function's insides is not here; what stands below is the part
# that needs only what a class or a docstring already tells, plus the
# numbers and words CPython fixes by name.

# The flags CPython sets on a compiled body. They are fixed numbers, and
# a program that only compares or combines them reads the right ones
# here. Nothing in this runtime hands out a co_flags to compare them
# against, so a test that reaches for one fails on that instead.
CO_OPTIMIZED = 1
CO_NEWLOCALS = 2
CO_VARARGS = 4
CO_VARKEYWORDS = 8
CO_NESTED = 16
CO_GENERATOR = 32
CO_NOFREE = 64
CO_COROUTINE = 128
CO_ITERABLE_COROUTINE = 256
CO_ASYNC_GENERATOR = 512

# The four words a generator's state is named by, and the same four for
# a coroutine.
GEN_CREATED = 'GEN_CREATED'
GEN_RUNNING = 'GEN_RUNNING'
GEN_SUSPENDED = 'GEN_SUSPENDED'
GEN_CLOSED = 'GEN_CLOSED'

CORO_CREATED = 'CORO_CREATED'
CORO_RUNNING = 'CORO_RUNNING'
CORO_SUSPENDED = 'CORO_SUSPENDED'
CORO_CLOSED = 'CORO_CLOSED'


def isclass(object):
    # A class is the one kind of thing this runtime can tell apart from
    # the rest without reading any insides.
    return isinstance(object, type)


def getmro(cls):
    return cls.__mro__


def cleandoc(doc):
    # Take the indentation a docstring picked up from the block it was
    # written in back off, the way CPython does: the first line loses
    # its leading space, and every line after loses the smallest
    # indentation any of them has.
    lines = doc.expandtabs().split('\n')
    margin = -1
    for line in lines[1:]:
        stripped = line.lstrip()
        if not stripped:
            continue
        indent = len(line) - len(stripped)
        if margin < 0 or indent < margin:
            margin = indent
    cleaned = [lines[0].strip()]
    if margin > 0:
        for line in lines[1:]:
            cleaned.append(line[margin:].rstrip())
    else:
        for line in lines[1:]:
            cleaned.append(line.rstrip())
    while cleaned and not cleaned[-1]:
        cleaned.pop()
    while cleaned and not cleaned[0]:
        cleaned.pop(0)
    return '\n'.join(cleaned)


def getdoc(object):
    if not hasattr(object, '__doc__'):
        return None
    doc = object.__doc__
    if doc is None:
        return None
    return cleandoc(doc)


def stack(context=1):
    # CPython walks the calls in progress and hands back a record for
    # each, with the frame, the file and the line. No kernel here keeps
    # the calls in progress as anything a program can reach, so an empty
    # list would read as a program called from nowhere, which is never
    # true. It refuses instead.
    raise 'NotImplementedError: inspect.stack needs the calls in progress as objects, which this runtime does not keep'


def currentframe():
    raise 'NotImplementedError: inspect.currentframe needs a frame object, which this runtime does not keep'


def getgeneratorstate(generator):
    # The four states are told apart by reading gi_frame and gi_running
    # off the generator. A generator here answers to neither, and the
    # difference between stopped at a yield and never started cannot be
    # seen from outside, so guessing one of the four would be a guess.
    raise 'NotImplementedError: inspect.getgeneratorstate needs a generator to carry its frame, which this runtime does not arrange'


def getcoroutinestate(coroutine):
    raise 'NotImplementedError: inspect.getcoroutinestate needs a coroutine to carry its frame, which this runtime does not arrange'


def __getattr__(name):
    raise 'NotImplementedError: inspect.' + name + ' needs to read a compiled body, which this runtime does not hand out'
