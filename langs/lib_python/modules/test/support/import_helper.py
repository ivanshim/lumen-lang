# Imports use the same cache as import statements.
def import_module(name, deprecated=False, required_on=None):
    return __load_module(name)

# This run keeps no module cache to isolate and carries no accelerator
# modules to block, so a fresh import is the import an import statement
# makes. As the reference does, None is handed back for a module that
# cannot be imported at all.
def import_fresh_module(name, fresh=(), blocked=(), deprecated=False, usefrozen=False):
    try:
        return __load_module(name)
    except ImportError:
        return None
