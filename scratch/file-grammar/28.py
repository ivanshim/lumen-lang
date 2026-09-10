print(value := 7)
print(value)
def identity(f): return f
@saved := identity
def answer(): return 42
print(answer())
again = saved(answer)
print(again())
