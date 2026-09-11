try:
    exec("1 +")
except SyntaxError as e:
    print(type(e).__name__)
try:
    eval("nosuch")
except NameError as e:
    print(type(e).__name__)
