from contextlib import redirect_stdout
from io import StringIO
s = StringIO()
with redirect_stdout(s):
    print('one')
    assert s.getvalue() == 'one\n'
    t = StringIO()
    with redirect_stdout(t):
        print('two')
    print('three')
print(s.getvalue(), end='')
print(t.getvalue(), end='')
