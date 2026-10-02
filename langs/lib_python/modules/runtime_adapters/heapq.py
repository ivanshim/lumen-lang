# Runtime adapter: the existing statistics module uses this private ordering helper.

def _ordered(iterable, key=None, reverse=False):
    values = []
    keys = []
    for item in iterable:
        wanted = item if key is None else key(item)
        at = len(values)
        while at > 0:
            before = wanted < keys[at - 1] if not reverse else keys[at - 1] < wanted
            if not before:
                break
            at -= 1
        values = [*values[:at], item, *values[at:]]
        keys = [*keys[:at], wanted, *keys[at:]]
    return values

