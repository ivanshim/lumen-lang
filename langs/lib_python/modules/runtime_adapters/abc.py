# Preserve ABC registration and propagate the native pattern protocol flags.
_abc_register_original = ABCMeta.register

def _abc_register_with_protocol(cls, subclass):
    result = _abc_register_original(cls, subclass)
    flags = getattr(cls, '__abc_tpflags__', 0)
    if flags:
        try:
            subclass.__abc_tpflags__ = flags
        except (AttributeError, TypeError):
            pass
    return result

ABCMeta.register = _abc_register_with_protocol
