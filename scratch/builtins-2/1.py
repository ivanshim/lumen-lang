exec("z = 3"); print(z)
ns = {}; exec("q = 5", ns); print(ns["q"])
print(eval("1 + 2"), eval("x + 1", {"x": 10}))
