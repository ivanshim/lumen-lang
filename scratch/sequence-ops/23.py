def remember(value, seen=[]):
    seen.append(value)
    return seen
first = remember(1)
print(first)
second = remember(2)
print(first, second, first is second)

def grow(items=[3]):
    items += [4]
    items *= 2
    return items
print(grow())
print(grow())
