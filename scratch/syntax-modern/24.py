print(r"a\nb\\c\"d")
print(R'''# kept ' and '' and \n
last''')
print(u"one\ntwo")
print(U"plain" " joined" r"\n")
print(("across "
       # The comment is between the literals.
       r"lines\n"))
r = 4
u = 5
print(r + u)
print("after")
