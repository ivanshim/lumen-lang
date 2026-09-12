from test.support import captured_stdout
with captured_stdout() as s:
    print("in")
print(repr(s.getvalue()))
