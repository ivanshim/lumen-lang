try:
    exec('print(1)', {'__builtins__': {}})
except NameError as e:
    print(type(e).__name__, e)
