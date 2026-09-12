# test_opcodes.py:49, a watched statement within a class.
class C:
    try:
        pass
    except NameError:
        pass
    x: int
