# A partial socket module: it carries the faults the module raises, and
# nothing that opens a connection. What a program asks of a real socket
# is absent, so hasattr says no rather than a wrong answer.
error = OSError

class gaierror(OSError):
    pass

class herror(OSError):
    pass

timeout = TimeoutError
