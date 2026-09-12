values = [3, 1, 2]
calls = []
def fail(v):
    calls.append(v)
    if v == 1:
        raise ValueError("key failed")
    return v
try:
    values.sort(key=fail)
except ValueError as e:
    print(str(e), values, calls)
values = [3, 1, 2]
def change(v):
    values.append(9)
    return v
try:
    values.sort(key=change)
except ValueError as e:
    print(str(e), values)
