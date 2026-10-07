# Protocols for native generic aliases and union values.

# Substitute each free parameter through its preparation and validation hooks.
def substitute(alias, supplied):
    from types import GenericAlias
    from typing import TypeVarTuple
    parameters = alias.__parameters__
    if not parameters:
        raise TypeError(f'{alias} is not a generic class')
    arguments = supplied if isinstance(supplied, tuple) else (supplied,)
    expanded = []
    for arg in arguments:
        unpacked = getattr(arg, '__typing_unpacked_tuple_args__', None)
        if unpacked is not None and not (len(unpacked) == 2 and unpacked[1] is Ellipsis):
            expanded.extend(unpacked)
        else:
            expanded.append(arg)
    arguments = tuple(expanded)
    for parameter in parameters:
        prepare = getattr(parameter, '__typing_prepare_subst__', None)
        if prepare is not None:
            arguments = prepare(alias, arguments if isinstance(arguments, tuple) else (arguments,))
    if not isinstance(arguments, tuple):
        arguments = (arguments,)
    if len(arguments) != len(parameters):
        relation = 'many' if len(arguments) > len(parameters) else 'few'
        raise TypeError(f'Too {relation} arguments for {alias}; actual {len(arguments)}, expected {len(parameters)}')
    # Parameter lookup uses identity, including unhashable protocol objects.
    def replacement(parameter):
        for index, cached in enumerate(parameters):
            if cached is parameter:
                return arguments[index]
        raise TypeError(f'argument {parameter!r} with __typing_subst__ was not found in __parameters__')

    # Preserve classes and recurse through the argument containers of Callable.
    def replace(original):
        if isinstance(original, type):
            return original
        if isinstance(original, (list, tuple)):
            entries = replace_all(original)
            return list(entries) if isinstance(original, list) else entries
        subst = getattr(original, '__typing_subst__', None)
        if subst is not None:
            return subst(replacement(original))
        nested = getattr(original, '__parameters__', ())
        if not isinstance(nested, tuple) or not nested:
            return original
        selected = []
        for parameter in nested:
            value = replacement(parameter)
            if isinstance(parameter, TypeVarTuple):
                selected.extend(value)
            else:
                selected.append(value)
        return original[tuple(selected)]

    # Unpacked substitution hooks must return a tuple before it can be expanded.
    def replace_all(originals):
        result = []
        for original in originals:
            changed = replace(original)
            if getattr(original, '__typing_is_unpacked_typevartuple__', False):
                if not isinstance(changed, tuple):
                    raise TypeError(f"expected __typing_subst__ of {type(original).__name__} objects to return a tuple, not {type(changed).__name__}")
                result.extend(changed)
            elif isinstance(original, GenericAlias) and original.__unpacked__:
                entries = changed.__typing_unpacked_tuple_args__
                if entries is not None and not (len(entries) == 2 and entries[1] is Ellipsis):
                    result.extend(entries)
                else:
                    result.append(changed)
            else:
                result.append(changed)
        return tuple(result)

    result = replace_all(alias.__args__)
    if isinstance(alias, GenericAlias):
        specialized = GenericAlias(alias.__origin__, tuple(result))
        return next(iter(specialized)) if alias.__unpacked__ else specialized
    from typing import Union
    return Union[tuple(result)]

# Fold a generic alias's origin and arguments exactly as its equality does.
def alias_hash(alias):
    return hash(alias.__origin__) ^ hash(alias.__args__)

# Reduce an ordinary alias to its public constructor and its actual arguments.
def alias_reduce(alias, protocol=None):
    from types import GenericAlias
    if alias.__unpacked__:
        return next, (iter(GenericAlias(alias.__origin__, alias.__args__)),)
    return type(alias), (alias.__origin__, alias.__args__)

# Protocol-specific reduction still dispatches the subclass reduction hook.
def alias_reduce_ex(alias, protocol):
    return alias.__reduce__()

# Parameter views compare only the same view kind and their actual origins.
def parameter_view_equal(view, other):
    if type(view) is not type(other):
        return NotImplemented
    return view.__origin__ == other.__origin__

# Compare unions as unordered alternatives, with duplicate members already folded.
def union_equal(union, other):
    from typing import Union
    if not isinstance(other, Union):
        return NotImplemented
    if getattr(union, '\0union-hashable') != getattr(other, '\0union-hashable'):
        return False
    left = getattr(union, '\0union-unhashable')
    right = getattr(other, '\0union-unhashable')
    return len(left) == len(right) and all(member in right for member in left) and all(member in left for member in right)

# Give unordered alternatives the same hash regardless of their insertion order.
def union_hash(union):
    unhashable = getattr(union, '\0union-unhashable')
    if unhashable:
        for member in unhashable:
            hash(member)
        raise TypeError(f'union contains {len(unhashable)} unhashable elements')
    return hash(getattr(union, '\0union-hashable'))

# Keep direct union reduction unavailable; pickle uses copyreg's registered reducer.
def union_reduce(union, protocol=None):
    if protocol is None:
        raise TypeError("cannot pickle 'Union' object")
    raise TypeError("cannot pickle 'typing.Union' object")

# Return the singleton by its public global name, as its reconstruction protocol asks.
def no_default_reduce(value, protocol=None):
    return 'NoDefault'

# Reconstruct named type parameters and aliases by their defining module's binding.
def parameter_reduce(value, protocol=None):
    return value.__name__

# Prepare a TypeVar's argument at its own position, including a lazy default.
def typevar_prepare(parameter, alias, arguments):
    index = alias.__parameters__.index(parameter)
    size = len(arguments)
    if index < size:
        return arguments
    if index == size:
        from typing import NoDefault
        default = parameter.__default__
        if default is not NoDefault:
            return arguments + (default,)
    raise TypeError(f'Too few arguments for {alias}; actual {size}, expected at least {index + 1}')

# Native alias display keeps nested aliases intact and qualifies class-like values.
def alias_repr(alias):
    def display(value):
        if value is type(None):
            return 'None'
        if value is Ellipsis:
            return '...'
        if hasattr(value, '__origin__') and hasattr(value, '__args__'):
            return repr(value)
        name = getattr(value, '__qualname__', None)
        module = getattr(value, '__module__', None)
        if name is not None and module is not None:
            return str(name) if module == 'builtins' else str(module) + '.' + str(name)
        return repr(value)
    def argument(value):
        if type(value) is list:
            return '[' + ', '.join(display(item) for item in value) + ']'
        return display(value)
    entries = ', '.join(argument(value) for value in alias.__args__) if alias.__args__ else '()'
    return ('*' if alias.__unpacked__ else '') + display(alias.__origin__) + '[' + entries + ']'

# ParamSpec views display their parameter name, or the repr of an arbitrary origin.
def parameter_view_display(origin, kind):
    from typing import ParamSpec
    name = origin.__name__ if isinstance(origin, ParamSpec) else repr(origin)
    return name + ('.args' if kind == 'ParamSpecArgs' else '.kwargs')

# Parameter objects cannot replace themselves with bases in a class declaration.
def reject_parameter_base(parameter, bases):
    raise TypeError(f'Cannot subclass an instance of {type(parameter).__name__}')

# A union of alternatives is also unavailable as a class base.
def reject_union_base(union, bases):
    raise TypeError(f'Cannot subclass {union}')

# Discover free parameters by the native alias protocol, excluding bare classes.
def alias_parameters(arguments):
    result = []
    for argument in arguments:
        if isinstance(argument, type):
            continue
        if hasattr(argument, '__typing_subst__'):
            found = (argument,)
        else:
            found = getattr(argument, '__parameters__', None)
            if found is None and isinstance(argument, (list, tuple)):
                found = alias_parameters(argument)
        if isinstance(found, tuple):
            for parameter in found:
                if not any(parameter is prior for prior in result):
                    result.append(parameter)
    return tuple(result)

# Keep distinct alternatives and their construction-time hashability separately.
def union_state(arguments):
    members = []
    hashable = set()
    unhashable = []
    for argument in arguments:
        try:
            hash(argument)
        except Exception:
            if argument in unhashable:
                continue
            unhashable.append(argument)
        else:
            if argument in hashable:
                continue
            hashable.add(argument)
        members.append(argument)
    return tuple(members), frozenset(hashable), tuple(unhashable)

# Render every union alternative through its own qualified name or representation.
def union_repr(union):
    pieces = []
    for member in union.__args__:
        if member is type(None):
            pieces.append('None')
        elif hasattr(member, '__origin__') and hasattr(member, '__args__'):
            pieces.append(repr(member))
        else:
            name = getattr(member, '__qualname__', None)
            module = getattr(member, '__module__', None)
            if name is not None and module is not None:
                pieces.append(name if module == 'builtins' else module + '.' + name)
            else:
                pieces.append(repr(member))
    return ' | '.join(pieces)


def reject_union_new(*args, **kwargs):
    raise TypeError("cannot create 'typing.Union' instances")
