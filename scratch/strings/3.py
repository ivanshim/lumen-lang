def show(s):
    print(f"{s!r}", len(s))
show("a" "b")
show(("a"
      "b"))
show("\x41\u0041\U00000041\101")
show("\a\b\f\v\0\r\n\t")
show("a\
b")
show(r"a\"b\\c")
show('''one "quote"
# two 'quotes'
three''')
show("\400\777")
show(f"\x7b\x7d")
