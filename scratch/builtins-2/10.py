print(eval('None'))
try:
    eval('1; 2')
except SyntaxError as e:
    print(type(e).__name__)
try:
    compile('1\n+ 2', '<s>', 'eval')
except SyntaxError as e:
    print(type(e).__name__)
print(eval(' (1 +\n2) '))
