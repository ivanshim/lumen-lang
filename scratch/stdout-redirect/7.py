import sys, io
s = io.StringIO()
s.close()
try:
    print('x', file=s)
except ValueError as error:
    print(str(error))
sys.stdin = s
try:
    input()
except ValueError as error:
    print(str(error))
sys.stdin = sys.__stdin__
