def outer(x):
    if False:
        class C:
            nonlocal x
            x += 1
    return x
print(outer(7))
