try:
    raise RuntimeError("a") from KeyError("b")
except RuntimeError as e:
    print(e.__cause__)
raise TypeError("t")
