# Subprocess helpers are named but cannot run a second interpreter.
def assert_python_ok(*args, **kwargs):
    raise 'NotImplementedError: interpreter subprocesses are not supported'

def assert_python_failure(*args, **kwargs):
    raise 'NotImplementedError: interpreter subprocesses are not supported'
