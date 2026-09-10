def f(n): return f(n - 1) if n else 0
try:
    f(5000)
except RecursionError:
    print("caught")
finally:
    print("cleaned")
print(f(500))
