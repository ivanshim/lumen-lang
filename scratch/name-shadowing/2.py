import os
import sys
sys.dont_write_bytecode = True
with open("name_shadowing_fixture.py", "w") as f:
    f.write('class Conditional:\n    if False: len = lambda a: 99\n    answer = len([1, 2])\nclass Deleted:\n    len = lambda a: 99\n    del len\n    answer = len([1, 2, 3])\ndef reader(): return sum(2, 3)\nsum = lambda a, b: a + b\nprint(Conditional.answer, Deleted.answer, reader())\n')
sys.path.insert(0, ".")
try:
    import name_shadowing_fixture
finally:
    os.remove("name_shadowing_fixture.py")
