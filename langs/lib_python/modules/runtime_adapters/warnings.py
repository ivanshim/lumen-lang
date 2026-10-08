# Preserve the runtime's private warning-capture interface for existing callers.
# Captures and restoration still use CPython's complete context implementation.
class catch_warnings(catch_warnings):
    # Accept the internal capture marker while keeping public arguments intact.
    def __init__(self, *, record=False, module=None, action=None,
                 category=Warning, lineno=0, append=False, _internal=False):
        super().__init__(record=record, module=module, action=action,
                         category=category, lineno=lineno, append=append)

    # Expose the same live list returned by the upstream recording context.
    def __enter__(self):
        self.records = super().__enter__()
        return self.records
