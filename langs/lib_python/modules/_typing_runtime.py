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
            arguments = prepare(alias, arguments)
    if len(arguments) != len(parameters):
        relation = 'many' if len(arguments) > len(parameters) else 'few'
        raise TypeError(f'Too {relation} arguments for {alias}; actual {len(arguments)}, expected {len(parameters)}')
    replacements = dict(zip(parameters, arguments))
    result = []
    for original in alias.__args__:
        if getattr(original, '__typing_is_unpacked_typevartuple__', False):
            changed = replacements[original.__args__[0]]
        elif hasattr(original, '__typing_subst__'):
            changed = original.__typing_subst__(replacements[original])
        else:
            nested = getattr(original, '__parameters__', ())
            if nested:
                selected = []
                for parameter in nested:
                    replacement = replacements[parameter]
                    if isinstance(parameter, TypeVarTuple):
                        selected.extend(replacement)
                    else:
                        selected.append(replacement)
                changed = original[tuple(selected)]
            else:
                changed = original
        if getattr(original, '__typing_is_unpacked_typevartuple__', False):
            result.extend(changed)
        elif isinstance(original, GenericAlias) and original.__unpacked__:
            entries = changed.__typing_unpacked_tuple_args__
            if entries is not None and not (len(entries) == 2 and entries[1] is Ellipsis):
                result.extend(entries)
            else:
                result.append(changed)
        else:
            result.append(changed)
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
    left, right = union.__args__, other.__args__
    return len(left) == len(right) and all(any(member == candidate for candidate in right) for member in left)

# Give unordered alternatives the same hash regardless of their insertion order.
def union_hash(union):
    return hash(frozenset(union.__args__))

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
