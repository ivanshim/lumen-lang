def check(obj, key):
    return obj.seq[key:], obj.seq[:key], obj.seq[::key]
print("read")
