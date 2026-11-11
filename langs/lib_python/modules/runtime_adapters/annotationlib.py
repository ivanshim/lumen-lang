# Runtime adapter derived from CPython v3.14.8 (8e6e75d9102e), Lib/annotationlib.py; PSF License.
# Template values render through the syntax tree the reference unparses.
def type_repr(value):
    """Convert a Python value to a format suitable for use with the STRING format.

    This is intended as a helper for tools that support the STRING format but do
    not have access to the code that originally produced the annotations. It uses
    repr() for most objects.

    """
    # Builtin functions are their own type here. isinstance does not see
    # them as types.BuiltinFunctionType, so the type object is compared.
    if isinstance(value, (type, types.FunctionType)) or type(value) is types.BuiltinFunctionType or type(value) is types.BuiltinMethodType:
        if value.__module__ == "builtins":
            return value.__qualname__
        return f"{value.__module__}.{value.__qualname__}"
    if _is_template(value):
        return _get_ast().unparse(_template_to_ast(value))
    if value is ...:
        return "..."
    return repr(value)


def get_annotate_from_class_namespace(obj):
    """Retrieve the annotate function from a class namespace dictionary.

    Return None if the namespace does not contain an annotate function.
    This is useful in metaclass ``__new__`` methods to retrieve the annotate function.
    """
    try:
        return obj["__annotate__"]
    except KeyError:
        return obj.get("__annotate_func__", None)


