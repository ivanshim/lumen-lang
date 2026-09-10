import re
print(re.fullmatch(r'(a|b){2,3}', 'aba').group(1))
print('%r' % (re.findall(r'[a-c]+', 'abc xx b'),))
print(re.search(r'\bcat\b', 'a cat!').group())
print(re.match(r'a.*?b', 'a1b2b').group())
print('%r' % (re.findall(r'.*?', 'a'),))
print(re.sub(r'x*', '-', 'ab'))
print('%r' % (re.split(r'(,)', 'a,b'),))
print(re.match('^b', 'ab') is None, re.search('b$', 'ab') is not None)
