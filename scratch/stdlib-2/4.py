from io import StringIO
with StringIO('ab\ncd') as s:
    print(s.readline(), end='')
    print(s.tell(), s.read())
    s.seek(1)
    print(s.write('X'), s.getvalue())
print(s.closed)
from contextlib import nullcontext, closing, ExitStack
with nullcontext(7) as value:
    print(value)
with ExitStack() as stack:
    first = stack.enter_context(StringIO())
    second = stack.enter_context(closing(StringIO()))
print(first.closed, second.closed)
