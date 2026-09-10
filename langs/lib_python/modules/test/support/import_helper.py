# Imports use the same cache as import statements.
def import_module(name, deprecated=False, required_on=None):
    return __load_module(name)

def import_fresh_module(name, fresh=None, blocked=None, deprecated=False, usefrozen=False):
    raise 'NotImplementedError: fresh module cache isolation is not supported'
