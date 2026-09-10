import sys, io
old = sys.stdin
sys.stdin = io.StringIO("alpha\nbeta\nlast")
print(input())
print(input("prompt: "))
print(input())
try:
    input()
except EOFError:
    print("eof")
sys.stdin = old
print(repr(sys.stdin.readline()))
print(repr(sys.stdin.read()))
s = io.StringIO("a\nb\nc")
print(repr(s.readline(1)))
print(repr(s.readline()))
print(repr(s.readlines()))
s = io.StringIO("first\nsecond\n")
for line in s:
    print(repr(line))
    break
print(repr(s.read()))
print(list(io.StringIO("x\ny\n")))
