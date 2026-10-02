# The asynchronous generator protocol, written in Python over the
# kernels' own generator machinery. An `async def` with a yield makes a
# generator whose protocol words (asend, athrow, aclose, __anext__,
# __aiter__) the kernels read through here: each hands back a coroutine
# that drives the generator when it is awaited, turning the walk's end
# into StopAsyncIteration as PEP 525 spells it. aclose does not yet
# police a generator that yields while it closes, the one corner of the
# protocol left aside.

async def _drive_send(gen, value):
    try:
        return gen.send(value)
    except StopIteration:
        raise StopAsyncIteration

async def _drive_throw(gen, args):
    try:
        return gen.throw(*args)
    except StopIteration:
        raise StopAsyncIteration

async def _drive_close(gen):
    gen.close()


def asend(gen, value):
    return _drive_send(gen, value)


def athrow(gen, *args):
    return _drive_throw(gen, args)


def aclose(gen):
    return _drive_close(gen)


def anext(gen):
    return _drive_send(gen, None)
