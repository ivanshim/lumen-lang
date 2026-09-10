match 7:
    case x if x > 3: print("guard", x)
match 9:
    case captured: print("capture", captured)
match 2:
    case 1 | 2: print("or")
match 0:
    case 8: print("wrong")
    case _: print("wildcard")
