# Text streams opened from a finalizer need the pure-Python backend while
# imports still work: shutdown clears them before finalizers run. Compiling
# that backend at import costs more than a second, and a process whose own
# finalizers only close binary streams never constructs one. A finalizer
# defined outside the I/O implementation is counted when its class is created
# or, for a class already in the program, when this module is imported; the
# backend and the encodings it looks up are prepared at exit only then.
import atexit
import builtins
import sys
import _io as _native_io

_finalizer_instances = 0
_build_class = builtins.__build_class__
_IO_MODULES = {"_io", "io", "_io_backend"}
_watched = set()


def _foreign_deleter(cls):
    seen = set()
    pending = [cls]
    while pending:
        current = pending.pop()
        if current in seen or current is object:
            continue
        seen.add(current)
        try:
            namespace = current.__dict__
        except AttributeError:
            namespace = None
        if namespace is not None:
            deleter = namespace.get("__del__")
            if deleter is not None:
                module = getattr(deleter, "__module__", None)
                return bool(module) and module not in _IO_MODULES
        pending.extend(getattr(current, "__bases__", ()))
    return False


def _watch(cls):
    if id(cls) in _watched or not _foreign_deleter(cls):
        return
    _watched.add(id(cls))
    orig = cls.__init__

    def init(self, *args, **kwargs):
        global _finalizer_instances
        result = orig(self, *args, **kwargs)
        _finalizer_instances += 1
        return result

    try:
        cls.__init__ = init
    except (AttributeError, TypeError):
        # A native class we cannot wrap may still run a finalizer.
        global _finalizer_instances
        _finalizer_instances += 1


def _note_existing():
    main = sys.modules.get("__main__")
    if main is None:
        return
    global _finalizer_instances
    for value in list(vars(main).values()):
        if isinstance(value, type):
            _watch(value)
        elif _foreign_deleter(type(value)):
            _finalizer_instances += 1


def _build_class_watching(func, name, *bases, **kwargs):
    cls = _build_class(func, name, *bases, **kwargs)
    _watch(cls)
    return cls


def _prime_text_runtime():
    if not _finalizer_instances:
        return
    # Constructing the stream loads the backend and caches the codec lookup
    # TextIOWrapper repeats after imports have been cleared.
    _native_io.TextIOWrapper(_native_io.BytesIO())
    _native_io.TextIOWrapper(_native_io.BytesIO(), encoding="utf-8", errors="strict")


builtins.__build_class__ = _build_class_watching
_note_existing()
atexit.register(_prime_text_runtime)
