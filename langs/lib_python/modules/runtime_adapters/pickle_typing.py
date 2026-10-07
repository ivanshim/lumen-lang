# Honor copyreg's extension reductions while retaining the runtime's byte writer.
_original_runtime_reduce = _reduce

# Resolve registered reducers before instance state and qualify named globals.
def _reduce(value, protocol):
    import copyreg
    reducer = copyreg.dispatch_table.get(type(value))
    result = reducer(value) if reducer is not None else _original_runtime_reduce(value, protocol)
    if isinstance(result, str):
        module = getattr(value, '__module__', None)
        if module is None:
            for name, namespace in list(sys.modules.items()):
                if namespace is not None and getattr(namespace, result, None) is value:
                    module = name
                    break
        try:
            found = _global(module, result) if module is not None else None
        except (KeyError, AttributeError, ImportError) as error:
            raise PicklingError(f"Can't pickle {value!r}: it's not found as {module}.{result}") from error
        if found is not value:
            raise PicklingError(f"Can't pickle {value!r}: it's not the same object as {module}.{result}")
        return _global, (module, result)
    return result
