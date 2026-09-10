"""A module's text may contain "quotes", '' pairs and # marks.
An escaped closer stays within it: \""" followed by more text.
"""
print("""first "pair" # kept
second 'quote'""
third""")
print('''one ' two '' # kept
three "four"''')
if True:
    text = """kept
  with its spaces"""
    print(text)
print("""""")
print("after")
