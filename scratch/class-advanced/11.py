parents = [super]
print(callable(parents[0]))
def keep():
    return super
print(callable(keep()))
