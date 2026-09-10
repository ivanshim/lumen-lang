def outer():
    items = [1]
    alias = items
    read = lambda: items
    saved = lambda value=items: value
    items[0] = 2
    items = [3]
    print(read(), saved(), alias)
    def replace():
        nonlocal items
        items = [4]
    replace()
    print(read(), saved(), alias)
    del items
    items = [5]
    print(read(), saved(), alias)
    return read, saved
read, saved = outer()
print(read(), saved())
def defaults():
    value = 6
    f = lambda x=value: x
    value = 7
    return f
print(defaults()())
