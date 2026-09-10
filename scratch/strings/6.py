print(f"{'same quote'}")
print(f"{f"{1 + 2}"}")
print(f"""{1 +
# a field may hold a comment
2}""")
print(f"{1 != 2} {1 == 2}")
print(f"{r'\n'!r}")
print(f"{b'\u0041\U00000041\N{A}'!r}")
print(f"{b'\400'!r}")
print(fr"\N{'A'}")
print(f"{'é'!a}")
print(f"{2:#x}")
def unread():
    return "\N{LATIN CAPITAL LETTER A}\ud800"
print("read past unavailable text")
print(f"{3!s  }")
print(f"{1+2 = # my comment
  }")
