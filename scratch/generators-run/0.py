def g(): yield 1; yield 2
for v in g(): print(v)
