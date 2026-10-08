# Bind MutableMapping.pop's class-local default in the module adapter scope.
# The canonical _collections_abc.py remains the unchanged CPython source.
_mutable_mapping_pop_marker = MutableMapping._MutableMapping__marker

def _mutable_mapping_pop(self, key, default=_mutable_mapping_pop_marker):
    try:
        value = self[key]
    except KeyError:
        if default is self._MutableMapping__marker:
            raise
        return default
    else:
        del self[key]
        return value

_mutable_mapping_pop.__doc__ = MutableMapping.pop.__doc__
_mutable_mapping_pop.__name__ = 'pop'
_mutable_mapping_pop.__qualname__ = 'MutableMapping.pop'
MutableMapping.pop = _mutable_mapping_pop

# Mark the native dict as a mapping for structural patterns.
Mapping.register(dict)
