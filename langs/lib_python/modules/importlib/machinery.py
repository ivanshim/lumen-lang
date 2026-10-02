# Metadata for source modules; bytecode and extension loaders are unavailable.
class ModuleSpec:
    def __init__(self, name, loader, *, origin=None, loader_state=None, is_package=None):
        self.name = name
        self.loader = loader
        self.origin = origin
        self.loader_state = loader_state
        self.submodule_search_locations = [] if is_package else None
        self.has_location = False
        self.cached = None

    @property
    def parent(self):
        if self.submodule_search_locations is not None:
            return self.name
        return self.name.rpartition('.')[0]
