from textwrap import dedent, indent, fill, wrap
from string import Template, ascii_letters, digits, punctuation, whitespace
print(dedent('  a\n  b'))
print(indent('a\n\nb', '> '))
print(fill('one two three', 7))
print(wrap('abcdefgh', 3))
print(Template('$who ${what} $$').substitute(who='a', what='b'))
print(Template('$missing').safe_substitute())
print(len(ascii_letters), len(digits), len(punctuation), len(whitespace))
