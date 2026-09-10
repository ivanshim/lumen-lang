match [1, 2, 3, 4]:
    case [a, b, *rest]: print(a, b, len(rest), rest[0], rest[1])
match [8]:
    case [x] as whole: print(x, len(whole), whole[0])
