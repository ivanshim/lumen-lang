# Native fast path for a complete top-level JSON string with default options.
# The pinned decoder handles hooks, alternate classes, and all error reports.
_lumen_python_loads = loads

def loads(s, *, cls=None, object_hook=None, parse_float=None, parse_int=None,
          parse_constant=None, object_pairs_hook=None, **kw):
    if cls is None and object_hook is None and parse_float is None and parse_int is None and parse_constant is None and object_pairs_hook is None and not kw:
        if type(s) is str and s.startswith('"'):
            result = __json_string_scan(s, 1)
            if result is not None:
                value, end = result
                if not s[end:].strip(' \t\n\r'):
                    return value
    return _lumen_python_loads(s, cls=cls, object_hook=object_hook,
                              parse_float=parse_float, parse_int=parse_int,
                              parse_constant=parse_constant,
                              object_pairs_hook=object_pairs_hook, **kw)

loads.__doc__ = _lumen_python_loads.__doc__
