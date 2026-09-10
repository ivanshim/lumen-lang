# Small managers use the ordinary entry and leaving methods.
class nullcontext:
    def __init__(self, enter_result=None):
        self.enter_result = enter_result

    def __enter__(self):
        return self.enter_result

    def __exit__(self, kind, value, traceback):
        return False

class closing:
    def __init__(self, thing):
        self.thing = thing

    def __enter__(self):
        return self.thing

    def __exit__(self, kind, value, traceback):
        self.thing.close()
        return False

class suppress:
    def __init__(self, *exceptions):
        self.exceptions = exceptions

    def __enter__(self):
        return None

    def __exit__(self, kind, value, traceback):
        for exception in self.exceptions:
            if isinstance(value, exception):
                return True
        return False

class redirect_stdout:
    def __init__(self, target):
        self.target = target
        self.saved = []

    def __enter__(self):
        import sys
        self.saved = [*self.saved, getattr(sys, 'stdout')]
        setattr(sys, 'stdout', self.target)
        return self.target

    def __exit__(self, kind, value, traceback):
        import sys
        setattr(sys, 'stdout', self.saved[len(self.saved) - 1])
        self.saved = self.saved[:-1]
        return False

# Stub: suspended generators cannot yet be resumed by the run.
class _GeneratorContextManager:
    def __init__(self, function, args, keywords):
        self.function = function
        self.args = args
        self.keywords = keywords

    def __enter__(self):
        self.function(*self.args, **self.keywords)
        raise 'NotImplementedError: generator context managers cannot be resumed'

    def __exit__(self, kind, value, traceback):
        return False

def contextmanager(function):
    def decorate(*args, **keywords):
        return _GeneratorContextManager(function, args, keywords)
    return decorate

class ExitStack:
    def __init__(self):
        self.exits = []

    def __enter__(self):
        return self

    def enter_context(self, manager):
        value = manager.__enter__()
        self.push(manager.__exit__)
        return value

    def push(self, exit):
        self.exits = [*self.exits, exit]
        return exit

    def callback(self, function, *args, **keywords):
        def leave(kind, value, traceback):
            function(*args, **keywords)
            return False
        self.push(leave)
        return function

    def pop_all(self):
        other = ExitStack()
        other.exits = self.exits
        self.exits = []
        return other

    def close(self):
        self.__exit__(None, None, None)

    def __exit__(self, kind, value, traceback):
        suppressed = False
        while len(self.exits):
            leave = self.exits[len(self.exits) - 1]
            self.exits = self.exits[:-1]
            if leave(kind, value, traceback):
                kind = None
                value = None
                traceback = None
                suppressed = True
        return suppressed
