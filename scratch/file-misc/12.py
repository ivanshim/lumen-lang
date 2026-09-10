global value
value = 1
def f():
    global value
    value = 3
f()
print(value)
