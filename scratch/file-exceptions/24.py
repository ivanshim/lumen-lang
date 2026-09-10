print([x if x > 1 else 0 for x in [1, 2] if x > 0])
print([x for x in ([1] if True else missing()) if (x if True else 0)])
