calls = []
def numbers():
    for v in [3, 1, 2]:
        calls.append(v)
        yield v
def key(v):
    calls.append(-v)
    return v
print(min(numbers(), key=key), calls)
class Key:
    def __call__(self, v):
        return -v
print(sorted([1, 3, 2], key=Key()))
