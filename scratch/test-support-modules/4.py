from test.support import captured_stdout, captured_stderr, swap_attr, bigmemtest, run_with_locale, sentinel, _1M, _1G, _2G, _4G
import sys
with captured_stdout() as out:
    print('held out')
with captured_stderr() as err:
    print('held err', file=sys.stderr)
print(out.getvalue(), end='')
print(err.getvalue(), end='')
print(_1M, _1G, _2G, _4G)
class Holder:
    value = 'old'
holder = Holder()
with swap_attr(holder, 'value', 'new'):
    print(holder.value)
print(holder.value)
@bigmemtest(size=2**32, memuse=1)
def small(self, size):
    print(size)
small(None)
