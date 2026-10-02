# Minimal importlib interface over the Python module loader.
def import_module(name, package=None):
    if name.startswith('.'):
        if not package:
            raise TypeError("the 'package' argument is required to perform a relative import")
        level = len(name) - len(name.lstrip('.'))
        parts = package.rsplit('.', level - 1)
        if len(parts) < level:
            raise ImportError('attempted relative import beyond top-level package')
        name = parts[0] + '.' + name[level:]
    return __load_module(name)
