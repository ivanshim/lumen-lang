e = ValueError("x")
e.add_note("n")
print(e.__notes__, e.with_traceback(None) is e)
import sys
try:
    1/0
except:
    print(sys.exc_info()[0].__name__)
