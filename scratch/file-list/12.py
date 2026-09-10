def targets(items):
    first, *rest = items
    *front, last = items
    for first, *rest in items:
        pass
    for (first, *rest), last in items:
        pass
    return (*items,)
print("read")
