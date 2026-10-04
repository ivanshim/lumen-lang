# Runtime implementation of CPython v3.14.8 Modules/atexitmodule.c; PSF License.
# The native end-of-run primitive invokes the dispatcher at interpreter shutdown.
import sys

_callbacks = []

def register(func, /, *args, **kwargs):
    if not callable(func):
        raise TypeError('the first argument must be callable')
    _callbacks.insert(0, (func, args, kwargs))
    return func

def unregister(func, /):
    i = len(_callbacks) - 1
    while i >= 0:
        item = _callbacks[i]
        if func == item[0]:
            j = min(len(_callbacks) - 1, i)
            while j >= 0:
                if _callbacks[j] is item:
                    del _callbacks[j]
                    i = j
                    break
                j -= 1
        if i >= len(_callbacks):
            i = len(_callbacks)
        i -= 1

def _clear():
    _callbacks.clear()

def _ncallbacks():
    return len(_callbacks)

def _run_exitfuncs():
    for func, args, kwargs in _callbacks[:]:
        try:
            func(*args, **kwargs)
        except BaseException as exc:
            try:
                sys.unraisablehook(sys.UnraisableHookArgs(type(exc), exc, exc.__traceback__,
                    'Exception ignored in atexit callback ' + repr(func), None))
            except BaseException as hook_error:
                sys.stderr.write('Exception ignored in sys.unraisablehook: ' + repr(sys.unraisablehook) + '\n')
                import traceback
                traceback.print_exception(type(hook_error), hook_error,
                    hook_error.__traceback__, file=sys.stderr, chain=False)
    _clear()

__at_end(_run_exitfuncs)
