def deferred_context():
    with manager() as value:
        return value

def deferred_loop():
    for i, (x, y) in sequence:
        if x == y:
            return i
        continue
print("read")
