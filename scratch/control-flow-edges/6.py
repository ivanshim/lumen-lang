def swallowed():
    try:
        raise ValueError("body")
    finally:
        return 2
print(swallowed())
def nested():
    try:
        try:
            return 3
        finally:
            print("inner")
    finally:
        print("outer")
print(nested())
for i in range(3):
    try:
        if i == 0:
            continue
        break
    finally:
        print("loop", i)
else:
    print("wrong")
for i in range(2):
    try:
        raise ValueError("discarded")
    finally:
        continue
else:
    print("continued")
while True:
    try:
        raise ValueError("discarded")
    finally:
        break
print("broken")
try:
    try:
        print("body")
    except ValueError:
        print("wrong")
    else:
        raise ValueError("else")
    finally:
        print("last")
except ValueError:
    print("outside")
saved = ValueError("same")
try:
    raise saved
except ValueError:
    try:
        raise
    except ValueError as e:
        print(e is saved)
