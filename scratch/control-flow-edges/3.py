for i in range(3):
    if i == 1:
        continue
    print(i)
else:
    print("end")
def f(n): return f(n - 1) if n else 0
print(f(500))
