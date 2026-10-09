# What the reference implementation's own internals answer about its
# objects. Only the probes this runtime can answer truthfully stand
# here; the rest refuse at the bottom of the file.

import signal as _signal

def has_inline_values(obj):
    # The kernel tracks the compact layout until dictionary replacement,
    # deletion, or growth beyond its available attribute places.
    return _has_inline_values(obj)


class SelfInterruptingContextManager:
    # The reference's own is written in C, where no evaluation check
    # stands between its words: it leaves an interrupt pending as it
    # enters, so the KeyboardInterrupt lands on the first statement of
    # the body and the leaving method runs all the same. The same is
    # arranged here by shape: the raise is the entering method's last
    # statement, so no statement's edge stands between the pending note
    # and the frame's end, and the interrupt is taken up as the body
    # begins.
    def __init__(self):
        self._within = False

    def __enter__(self):
        self._within = True
        _signal.raise_signal(_signal.SIGINT)

    def __exit__(self, *args):
        self._within = False
        return False

    def within(self):
        return self._within


# The reference's own module reports this many runs before a
# specialization is made; the same number stands here so the runs the
# tests make of it run as they run there.
SPECIALIZATION_THRESHOLD = 2


def get_recursion_depth():
    import sys
    frame = sys._getframe(1)
    depth = 0
    while frame is not None:
        depth += 1
        frame = frame.f_back
    return depth


def __getattr__(name):
    raise AttributeError("module '_testinternalcapi' has no attribute '" + name + "'")


# These code analyses read the kernels' native instructions and scope layout.
def _code_subject(value):
    import types
    if isinstance(value, types.CodeType):
        return value, None
    if isinstance(value, types.FunctionType):
        return value.__code__, value
    raise TypeError('argument must be a code object or a function')


def code_returns_only_none(code, /):
    import types
    if not isinstance(code, types.CodeType):
        raise TypeError('argument must be a code object')
    return not bool(code.co_flags & (0x20 | 0x80 | 0x100 | 0x200)) and __trace_native__(3, code)[2]


def get_co_localskinds(value, /):
    import types
    if not isinstance(value, types.CodeType):
        raise TypeError('argument must be a code object')
    code = value
    kinds = {}
    argcount = code.co_argcount
    keyword_end = argcount + code.co_kwonlyargcount
    for index, name in enumerate(code.co_varnames):
        kind = 0x20
        if index < code.co_posonlyargcount:
            kind |= 0x02
        elif index < argcount:
            kind |= 0x06
        elif index < keyword_end:
            kind |= 0x04
        elif index == keyword_end and code.co_flags & 4:
            kind |= 0x0a
        elif index == keyword_end + bool(code.co_flags & 4) and code.co_flags & 8:
            kind |= 0x0c
        kinds[name] = kind
    for name in code.co_cellvars:
        kinds[name] = kinds.get(name, 0) | 0x40
    for name in code.co_freevars:
        kinds[name] = 0x80
    return kinds


_UNSPECIFIED_CODE_NAMESPACE = object()


def _code_namespace(value, position):
    if value is not _UNSPECIFIED_CODE_NAMESPACE and not isinstance(value, dict):
        actual = 'None' if value is None else type(value).__name__
        raise TypeError('get_code_var_counts() argument ' + str(position) + ' must be dict, not ' + actual)


def get_code_var_counts(code, globalnames=_UNSPECIFIED_CODE_NAMESPACE,
                        attrnames=_UNSPECIFIED_CODE_NAMESPACE,
                        globalsns=_UNSPECIFIED_CODE_NAMESPACE,
                        builtinsns=_UNSPECIFIED_CODE_NAMESPACE):
    for namespace, position in ((globalsns, 4), (builtinsns, 5)):
        _code_namespace(namespace, position)
    code, function = _code_subject(code)
    kinds = get_co_localskinds(code)
    references, attributes, _ = __trace_native__(3, code)
    for names, label in ((globalnames, 'globalnames'), (attrnames, 'attrnames')):
        if names is not _UNSPECIFIED_CODE_NAMESPACE and not isinstance(names, set):
            raise TypeError('expected a set for "' + label + '", got ' + repr(names))
    if globalnames is _UNSPECIFIED_CODE_NAMESPACE:
        globalnames = set()
    if attrnames is _UNSPECIFIED_CODE_NAMESPACE:
        attrnames = set()
    references, attributes = set(references), set(attributes)
    globals_ = references - globalnames
    attrs = attributes - attrnames
    duplicates = len(((references | globalnames) & (attributes | attrnames)) & (globals_ | attrs))
    globalnames.update(references)
    attrnames.update(attributes)
    if globalsns is _UNSPECIFIED_CODE_NAMESPACE:
        globalsns = {} if function is None else function.__globals__
    if builtinsns is _UNSPECIFIED_CODE_NAMESPACE:
        builtinsns = {} if function is None else function.__builtins__
    args = code.co_argcount + code.co_kwonlyargcount + bool(code.co_flags & 4) + bool(code.co_flags & 8)
    argnames = code.co_varnames[:args]
    argcells = sum(name in code.co_cellvars for name in argnames)
    cells = len(code.co_cellvars)
    pure = len(code.co_varnames) - args
    othercells = cells - argcells
    locals_ = args + pure + othercells
    global_count = sum(name in globalsns for name in globals_)
    builtin_count = sum(name not in globalsns and name in builtinsns for name in globals_)
    unbound = len(code.co_names) + duplicates
    unknown = unbound - len(globals_) - len(attrs)
    return {
        'total': locals_ + len(code.co_freevars) + unbound,
        'locals': {
            'total': locals_,
            'args': {'total': args, 'numposonly': code.co_posonlyargcount,
                     'numposorkw': code.co_argcount - code.co_posonlyargcount,
                     'numkwonly': code.co_kwonlyargcount,
                     'varargs': int(bool(code.co_flags & 4)),
                     'varkwargs': int(bool(code.co_flags & 8))},
            'numpure': pure,
            'cells': {'total': cells, 'numargs': argcells, 'numothers': othercells},
            'hidden': {'total': sum(bool(kind & 0x10) for kind in kinds.values()),
                       'numpure': sum(bool(kind & 0x10) and not bool(kind & 0x40) for kind in kinds.values()),
                       'numcells': sum(bool(kind & 0x10) and bool(kind & 0x40) for kind in kinds.values())},
        },
        'numfree': len(code.co_freevars),
        'unbound': {
            'total': unbound,
            'numattrs': len(attrs), 'numunknown': unknown,
            'globals': {'total': len(globals_), 'numglobal': global_count,
                        'numbuiltin': builtin_count,
                        'numunknown': len(globals_) - global_count - builtin_count},
        },
    }


def verify_stateless_code(code, globalnames=_UNSPECIFIED_CODE_NAMESPACE,
                          globalsns=_UNSPECIFIED_CODE_NAMESPACE,
                          builtinsns=_UNSPECIFIED_CODE_NAMESPACE):
    if globalnames is not _UNSPECIFIED_CODE_NAMESPACE and not isinstance(globalnames, set):
        actual = 'None' if globalnames is None else type(globalnames).__name__
        raise TypeError('get_code_var_counts() argument 2 must be set, not ' + actual)
    for namespace, position in ((globalsns, 3), (builtinsns, 4)):
        _code_namespace(namespace, position)
    code, function = _code_subject(code)
    subject = code if function is None else function
    counts = get_code_var_counts(subject, globalnames=globalnames,
                                 globalsns=globalsns, builtinsns=builtinsns)
    if code.co_freevars:
        raise ValueError('closures not supported')
    globals_ = counts['unbound']['globals']
    has_builtins = function is not None or builtinsns is not _UNSPECIFIED_CODE_NAMESPACE
    if globals_['numglobal'] or (has_builtins and globals_['numunknown']):
        raise ValueError('globals not supported')
    return None
