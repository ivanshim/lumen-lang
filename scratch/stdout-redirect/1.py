import sys
print("e", file=sys.stderr)
n = sys.stdout.write("ab\n")
print(n)
sys.stdout = None
print("x")
sys.stdout = sys.__stdout__
print("back")
