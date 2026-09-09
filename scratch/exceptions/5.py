def f():
    try:
        return 7
    finally:
        print("return-finally")
print(f())
def g():
    try:
        return 7
    finally:
        return 9
print(g())
try:
    raise "outer"
except:
    try:
        raise "inner"
    except:
        print("inner")
    try:
        raise
    except:
        print("restored")
