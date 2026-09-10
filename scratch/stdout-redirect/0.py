import sys, io
buf = io.StringIO()
old = sys.stdout
sys.stdout = buf
print("hidden")
sys.stdout = old
print("got:", buf.getvalue().strip())
