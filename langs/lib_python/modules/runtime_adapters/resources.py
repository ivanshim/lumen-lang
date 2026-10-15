# Native source modules have no loader spec; their resources live by __file__.
_loader_resources = from_package

# Read source-backed resources while retaining loaders supplied by callers.
def from_package(package):
    if package.__spec__ is None and getattr(package, '__file__', None):
        return pathlib.Path(package.__file__).parent
    return _loader_resources(package)
