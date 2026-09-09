# Arrays and maps copy when written. Objects need their own member cells.
def copy(value):
    return __copy_value(value, False)

def deepcopy(value, memo=None):
    if memo is not None:
        raise 'NotImplementedError: an external deepcopy memo is not supported'
    return __copy_value(value, True)
