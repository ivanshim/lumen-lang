# Text streams opened from a finalizer need the pure-Python backend while
# imports still work: shutdown clears them before finalizers run. Compiling
# that backend at import costs more than a second, and a process with no
# foreign finalizer instances never constructs one at shutdown.
import atexit
import sys
import _io as _native_io

# Modules whose finalizers only release native handles or close binary streams.
_IO_MODULES = {
    "_io", "io", "_io_backend", "_io_buffered",
    "zlib", "_bz2", "_lzma", "posix", "gzip", "tarfile",
}


def _foreign_deleter(cls):
    if cls is type or cls is object or not isinstance(cls, type):
        return False
    seen = set()
    pending = [cls]
    while pending:
        current = pending.pop()
        cid = id(current)
        if cid in seen or current is object or not isinstance(current, type):
            continue
        seen.add(cid)
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


_PLAIN_TYPE_IDS = {
    id(t) for t in (
        type, object, str, int, float, bool, bytes, bytearray, complex,
        type(None), list, tuple, dict, set, frozenset, slice, range,
        type(_foreign_deleter), type(len), type(sys), property,
        classmethod, staticmethod,
    )
}


def _has_foreign_finalizer():
    checked = {}
    plain = _PLAIN_TYPE_IDS

    def want(obj):
        cls = type(obj)
        cid = id(cls)
        if cid in plain:
            return False
        hit = checked.get(cid)
        if hit is None:
            hit = _foreign_deleter(cls)
            checked[cid] = hit
        return hit

    for name, mod in list(sys.modules.items()):
        if mod is None or name in _IO_MODULES:
            continue
        try:
            ns = vars(mod)
        except TypeError:
            continue
        is_main = name == "__main__"
        for value in list(ns.values()):
            cls = type(value)
            cid = id(cls)
            if cid not in plain:
                hit = checked.get(cid)
                if hit is None:
                    hit = _foreign_deleter(cls)
                    checked[cid] = hit
                if hit:
                    return True
            elif is_main:
                if (cls is list or cls is tuple or cls is set) and len(value) <= 64:
                    for item in value:
                        if want(item):
                            return True
                elif cls is dict and value is not ns and len(value) <= 64:
                    for item in value.values():
                        if want(item):
                            return True
    return False


def _prime_text_runtime():
    if not _has_foreign_finalizer():
        return
    # Constructing the stream loads the backend and caches the codec lookup
    # TextIOWrapper repeats after imports have been cleared.
    _native_io.TextIOWrapper(_native_io.BytesIO())
    _native_io.TextIOWrapper(_native_io.BytesIO(), encoding="utf-8", errors="strict")


atexit.register(_prime_text_runtime)
