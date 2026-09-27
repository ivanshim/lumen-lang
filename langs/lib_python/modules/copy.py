# Arrays and maps copy when written. Objects need their own member cells.
def copy(value):
    return __copy_value(value, False)

def deepcopy(value, memo=None):
    if memo is not None:
        raise 'NotImplementedError: an external deepcopy memo is not supported'
    copier = getattr(value, '__deepcopy__', None)
    if copier is not None:
        return copier({})
    if isinstance(value, (set, frozenset)):
        members = [deepcopy(item) for item in value]
        return type(value)(members)
    return __copy_value(value, True)
