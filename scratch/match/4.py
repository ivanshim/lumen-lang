match None:
    case None: print("none")
match True:
    case 1: print("numeric boolean")
    case _: print("wrong numeric boolean")
match 1:
    case True: print("wrong singleton")
    case 1: print("singleton distinct")
match -2:
    case -2: print("negative")
match "ab":
    case "a" "b": print("text")
match [1, 2, 3]:
    case [x, 8, *rest]: print("wrong sequence")
    case [x, *rest, y] if x > 5: print("wrong guard")
    case [x, *rest, y]: print(x, len(rest), y)
match []:
    case []: print("empty")
match [7]:
    case [7, 8]: print("wrong length")
    case [x, *rest]: print(x, len(rest))
match (None, 6):
    case (_, x): print("tuple", x)
def match(x):
    return x + 1
print(match(4))
case = 9
print(case)
