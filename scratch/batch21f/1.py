import functools
import inspect
import time
import unittest

answer = 42

class Globals(unittest.TestCase):
    # Read this module's globals after unittest calls across module boundaries.
    def test_module_globals(self):
        self.assertEqual(answer, 42)
        self.assertEqual(time.__name__, 'time')

result = unittest.TestResult()
Globals('test_module_globals').run(result)
assert not result.errors and not result.failures

# Recognize async callables after both kinds of functools wrapping.
async def coro(self, value=0):
    return value

class Methods:
    method = functools.partialmethod(coro, 1)

assert inspect.iscoroutinefunction(functools.partial(coro, None))
assert inspect.iscoroutinefunction(Methods.method)
assert inspect.iscoroutinefunction(Methods().method)

# Preserve an explicit coroutine marker through partial wrapping.
@inspect.markcoroutinefunction
def marked():
    return coro(None)

assert inspect.iscoroutinefunction(functools.partial(marked))
print('batch21f globals and coroutine recognition OK')
