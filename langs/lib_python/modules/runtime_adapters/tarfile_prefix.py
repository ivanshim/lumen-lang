# Command-line children each import this module from a cold process. The
# modules below are not needed to test or list an archive, so a stand-in
# satisfies the import until a real attribute is used.
def _lumen_install_cli_stubs():
    if __name__ != "__main__":
        return
    import sys
    if ("functools" in sys.modules and "shutil" in sys.modules
            and "gettext" in sys.modules):
        return

    class _Lazy:
        def __init__(self, name, early):
            object.__setattr__(self, "_n", name)
            object.__setattr__(self, "_r", None)
            object.__setattr__(self, "_e", dict(early))
            object.__setattr__(self, "_busy", False)

        def _load(self):
            real = object.__getattribute__(self, "_r")
            if real is not None:
                return real
            if object.__getattribute__(self, "_busy"):
                raise AttributeError("loading")
            object.__setattr__(self, "_busy", True)
            name = object.__getattribute__(self, "_n")
            try:
                sys.modules.pop(name, None)
                real = __import__(name)
            finally:
                object.__setattr__(self, "_busy", False)
            object.__setattr__(self, "_r", real)
            sys.modules[name] = self
            return real

        def __getattr__(self, attr):
            real = object.__getattribute__(self, "_r")
            if real is not None:
                return getattr(real, attr)
            early = object.__getattribute__(self, "_e")
            if attr in early:
                return early[attr]
            if attr[:1] == "_":
                raise AttributeError(attr)
            return getattr(self._load(), attr)

        def __setattr__(self, attr, value):
            real = object.__getattribute__(self, "_r")
            if real is None:
                object.__getattribute__(self, "_e")[attr] = value
            else:
                setattr(real, attr, value)

    def _lru(maxsize=128, typed=False):
        if callable(maxsize) and not isinstance(maxsize, bool):
            return _wrap(maxsize, 128, False)
        def deco(fn):
            return _wrap(fn, maxsize, typed)
        return deco

    def _wrap(fn, maxsize, typed):
        cache = {}
        def wrapper(*args, **kw):
            key = args
            if kw:
                key = args + tuple(sorted(kw.items()))
            if typed:
                key = key + tuple(type(v) for v in key)
            try:
                return cache[key]
            except KeyError:
                result = fn(*args, **kw)
                if maxsize != 0:
                    if maxsize is not None and len(cache) >= maxsize:
                        cache.pop(next(iter(cache)))
                    cache[key] = result
                return result
        wrapper.__wrapped__ = fn
        def _clear():
            cache.clear()
        wrapper.cache_clear = _clear
        return wrapper

    def _gettext(message):
        return message

    def _ngettext(singular, plural, n):
        return singular if n == 1 else plural

    def _put(name, early):
        if name not in sys.modules:
            sys.modules[name] = _Lazy(name, early)

    _put("functools", {"lru_cache": _lru, "__name__": "functools"})
    _put("shutil", {"__name__": "shutil"})
    _put("gettext", {"gettext": _gettext, "ngettext": _ngettext, "__name__": "gettext"})

_lumen_install_cli_stubs()
del _lumen_install_cli_stubs
