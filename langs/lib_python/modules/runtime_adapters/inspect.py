# Runtime adapter for partialmethod handles retained by the kernels.
# CPython v3.14.8 otherwise supplies the complete inspect implementation.
def _has_code_flag(f, flag):
    """Return true if ``f`` is a function (or a method or functools.partial
    wrapper wrapping a function or a functools.partialmethod wrapping a
    function) whose code object has the given ``flag``
    set in its flags."""
    f = _unwrap_coroutine_callable(f)
    while ismethod(f):
        f = f.__func__
    f = functools._unwrap_partial(f)
    if not (isfunction(f) or _signature_is_functionlike(f)):
        return False
    # If it's a pure Python function, or an object that is duck type
    # of a Python function (Cython and Mock functions, for instance), then:
    return bool(f.__code__.co_flags & flag)

def _has_coroutine_mark(f):
    f = _unwrap_coroutine_callable(f)
    while ismethod(f):
        f = f.__func__
    f = functools._unwrap_partial(f)
    return getattr(f, "_is_coroutine_marker", None) is _is_coroutine_mark

def _unwrap_coroutine_callable(func):
    # Stop when no wrapper was removed. Native bound-method reads may create
    # equivalent handles, so their identity is not a termination condition.
    while True:
        method = getattr(func, '__partialmethod__', None)
        if isinstance(method, functools.partialmethod):
            func = method.func
        elif isinstance(func, (functools.partialmethod, functools.partial)):
            func = func.func
        else:
            return func


# Native disassembly has no LOAD_GLOBAL/LOAD_ATTR operands to classify yet.
_getclosurevars_from_source = getclosurevars

def getclosurevars(func):
    """Resolve cells; refuse global analysis without instruction reference kinds."""
    if ismethod(func):
        func = func.__func__
    if isfunction(func) and func.__code__.co_names:
        raise NotImplementedError('native instruction metadata cannot distinguish global and attribute references')
    return _getclosurevars_from_source(func)


# Retained frame offsets index native instructions, not two-byte CPython words.
def _get_code_position(code, instruction_index):
    if instruction_index < 0:
        return (None, None, None, None)
    return next(instruction.positions for instruction in dis.get_instructions(code)
                if instruction.offset == instruction_index)


# Native annotation evaluators cannot resolve every FORWARDREF expression yet.
_signature_from_function_source = _signature_from_function

def _signature_from_function(cls, func, skip_bound_arg=True,
                             globals=None, locals=None, eval_str=False,
                             *, annotation_format=Format.VALUE):
    try:
        return _signature_from_function_source(
            cls, func, skip_bound_arg, globals, locals, eval_str,
            annotation_format=annotation_format)
    except (NameError, NotImplementedError) as error:
        if (annotation_format != Format.FORWARDREF
                or not isinstance(error.__context__, NotImplementedError)):
            raise
        raise ValueError('native annotation metadata cannot provide a forward-reference signature') from error
