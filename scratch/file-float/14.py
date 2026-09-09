def continued():
    for c in [1]:
        with manager:
            if not c:
                continue

def departed():
    for c in [1]:
        with manager:
            break

def returned():
    with manager:
        return 1

def unpacked():
    for x, y in [(1, 2)]:
        return x

print("read")
