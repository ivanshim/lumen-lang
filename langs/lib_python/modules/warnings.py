# Warning emission and filters are stubs; no warning text is printed.
def warn(message, category=None, stacklevel=1, source=None):
    pass

def simplefilter(action, category=None, lineno=0, append=False):
    pass

class catch_warnings:
    def __init__(self, record=False, module=None, action=None, category=None, lineno=0, append=False):
        self.record = record

    def __enter__(self):
        if self.record:
            return []
        return None

    def __exit__(self, kind, value, traceback):
        return False
