def g():
    yield 1
it = g()
print(next(it))
it.throw("problem")
