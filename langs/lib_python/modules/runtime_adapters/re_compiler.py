# Runtime adapter: re._compiler.compile with a native compiler tried first, a pattern it accepts
# skipping the Python parser. The body below that is CPython v3.14.8 Lib/re/_compiler.py compile()
# unchanged (PSF License); it stays here, in the same frame, so that the warnings it raises name
# the same caller as before.
def compile(p, flags=0):
    found = _sre.fast_compile(p, flags)
    if found is not None:
        return found
    # internal: convert pattern list to internal format

    if isstring(p):
        pattern = p
        p = _parser.parse(p, flags)
    else:
        pattern = None

    code = _code(p, flags)

    if flags & SRE_FLAG_DEBUG:
        print()
        dis(code)

    # map in either direction
    groupindex = p.state.groupdict
    indexgroup = [None] * p.state.groups
    for k, i in groupindex.items():
        indexgroup[i] = k

    return _sre.compile(
        pattern, flags | p.state.flags, code,
        p.state.groups-1,
        groupindex, tuple(indexgroup)
        )
