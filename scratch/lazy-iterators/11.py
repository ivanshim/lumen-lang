it = iter([])
try:
    next(it)
except StopIteration:
    print("stopped")
print(next(it, "still stopped"))
