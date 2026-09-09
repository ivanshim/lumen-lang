match [1, 2, 3]:
    case *rest, last: print(len(rest), last)
match 4,:
    case (x,): print(x)
match -0x10:
    case -0x10: print("hex")
match 1.5:
    case 1.5: print("real")
match [2, 8]:
    case [1, x] | [2, x]: print("alternative", x)
def shapes(value):
    match value:
        case {"key": bound}: pass
        case Namespace(key=bound): pass
        case Outer.Inner([x], key={"k": y}, other=z): pass
print("read shapes")
