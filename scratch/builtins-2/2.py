code = compile("a * 2", "<s>", "eval"); a = 4; print(eval(code))
def f():
    v = 1
    return sorted(locals())
print(f())
