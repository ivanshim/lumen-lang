try:
    hash(slice(1, 2, []))
except TypeError:
    print("unhashable bound")
