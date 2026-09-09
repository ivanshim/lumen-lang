# Arrays and maps copy when written. Objects need their own member cells.
def copy(value):
    return __copy_value(value, False)

def deepcopy(value, memo=None):
    return __copy_value(value, True)
