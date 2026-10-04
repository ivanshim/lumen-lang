# Basic warning delivery for modules without a configured logging system.
class Logger:
    def __init__(self, name):
        self.name = name

    def warning(self, msg, *args, **kwargs):
        import sys
        if args:
            msg = msg % args
        sys.stderr.write(str(msg) + '\n')


def getLogger(name=None):
    return Logger(name)
