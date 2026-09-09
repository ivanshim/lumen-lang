def decorate(cls):
    return cls

@decorate
class C:
    pass

print(42)
