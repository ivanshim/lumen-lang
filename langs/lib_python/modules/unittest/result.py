# Test outcomes shared by unittest runners and suites.
class TestResult:
    def __init__(self):
        self.testsRun = 0
        self.failures = []
        self.errors = []
        self.skipped = []
        self.expectedFailures = []
        self.unexpectedSuccesses = []

    def wasSuccessful(self):
        return len(self.failures) == 0 and len(self.errors) == 0 and len(self.unexpectedSuccesses) == 0

    def __repr__(self):
        return '<%s.%s run=%d errors=%d failures=%d>' % (
            self.__class__.__module__, self.__class__.__qualname__,
            self.testsRun, len(self.errors), len(self.failures))
