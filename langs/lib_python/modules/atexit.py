# Runtime adapter for CPython v3.14.8 Modules/atexitmodule.c (PSF License).
# The existing end-of-program hook invokes callbacks in reverse registration order.
_callbacks = []

def register(func, /, *args, **kwargs):
    if not callable(func):
        raise TypeError('the first argument must be callable')
    _callbacks.append((func, args, kwargs))
    return func

def unregister(func, /):
    global _callbacks
    _callbacks = [entry for entry in _callbacks if entry[0] != func]

def _clear():
    _callbacks.clear()

def _ncallbacks():
    return len(_callbacks)

def _run_exitfuncs():
    pending = list(reversed(_callbacks))
    _callbacks.clear()
    for func, args, kwargs in pending:
        try:
            func(*args, **kwargs)
        except BaseException as error:
            import sys
            import traceback
            print('Exception ignored in atexit callback ' + repr(func) + ':', file=sys.stderr)
            traceback.print_exception(type(error), error, error.__traceback__)

__at_end__(_run_exitfuncs)
