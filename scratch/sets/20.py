def later():
    raise RuntimeError("later")


def g(x):
    if x == 2:
        raise RuntimeError("later")
    return []


def show(label, thunk):
    try:
        thunk()
    except BaseException as e:
        print(label, type(e).__name__, str(e))
    else:
        print(label, "ok")


# A set literal reads its members left to right and rejects an unhashable
# one; a raising call that stands ahead of it is seen first.
show("LIT1", lambda: {1, []})
show("LIT2", lambda: {later(), []})

# set.add and a set comprehension insert and hash each value at once, so an
# unhashable value that precedes a raising call is rejected before it runs.
def add_then_later():
    box = set()
    box.add([])
    later()


show("ADD", add_then_later)
show("COMP", lambda: {g(x) for x in [1, 2]})

# A row holds an unhashable member without complaint; its later call runs.
show("ROW", lambda: [[], later()])
