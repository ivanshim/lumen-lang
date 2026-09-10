def subject():
    print("subject")
    return [1, 2]
match subject():
    case [a, b] if a > 3: print("wrong guard")
    case [a, b]: print(a, b); print("same body")
    case _: print("wrong later case")
x = 99
match [1, 9]:
    case [x, 8]: print("wrong partial")
    case _: print("unchanged", x)
for item in range(1, 4):
    match item:
        case 2: continue
        case _: print("loop", item)
def answer(value):
    match value:
        case 3: return "returned"
        case _: return "other"
print(answer(3))
