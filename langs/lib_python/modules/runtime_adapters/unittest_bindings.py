# The native runner does not need the full CPython case dependency chain.
def __getattr__(name):
    if name == 'case':
        return _host_load_module('unittest.case')
    if name == 'result':
        return _host_load_module('unittest.result')
    if name == 'TestResult':
        return _host_load_module('unittest.result').TestResult
    raise AttributeError("module 'unittest' has no attribute '" + name + "'")


def __dir__():
    return sorted(set(globals()) | {'TestResult', 'case', 'result'})
