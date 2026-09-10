# Stub: this reference keeps the object alive. Calling it gives that
# object back; collection callbacks cannot be honoured by this stand-in.
class _Reference:
    def __init__(self, value):
        self.value = value

    def read(self):
        return self.value

def ref(value, callback=None):
    if callback is not None:
        raise 'NotImplementedError: weak reference callbacks are not supported'
    return _Reference(value).read
