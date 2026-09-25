# A stand-in for the reference debugger. No interactive prompt is
# offered here; the one function programs and tests reach for is kept
# so that `breakpoint()`'s default hook, and code that names it
# directly, both have something to call and something to patch out.
def set_trace(*args, **kwargs):
    pass
