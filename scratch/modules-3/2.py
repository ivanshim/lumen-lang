import traceback
try:
    1/0
except ZeroDivisionError:
    print(traceback.format_exc().splitlines()[-1])
import platform
print(platform.python_implementation())
