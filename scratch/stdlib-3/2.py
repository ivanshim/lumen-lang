import string, pprint
print(string.capwords("ab cd"), string.Template("$x!").substitute(x=1))
pprint.pprint({"k": [1, 2], "z": (3,)})
