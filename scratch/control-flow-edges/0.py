def f():
    try: return 1
    finally: print("fin")
print(f())
def g():
    try: return 1
    finally: return 2
print(g())
