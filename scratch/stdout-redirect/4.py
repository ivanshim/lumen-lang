import sys, io
from contextlib import redirect_stdout
from test.support import captured_stderr
buf = io.StringIO()
with redirect_stdout(buf):
    print("a", "b", sep="/", end="!", file=None, flush=True)
    print("direct", file=sys.__stdout__)
print(repr(buf.getvalue()))
with captured_stderr() as errors:
    print("error", file=sys.stderr)
print(repr(errors.getvalue()))
class Writer:
    def __init__(self):
        self.text = ''
        self.count = 0
    def write(self, text):
        self.text += text
    def flush(self):
        self.count += 1
w = Writer()
print("one", file=w, flush=True)
print("two", file=w, flush=False)
print(repr(w.text), w.count)
