print(4 if True else unknown())
print(unknown() if False else 5)
print(sum(x for x in range(5) if x > 1 if x < 4))
