text = """first
second"""
print(text)
x = 1
def f():
    global x
    x = 2
f()
print(x)
