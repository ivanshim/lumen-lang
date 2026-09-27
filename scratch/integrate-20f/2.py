assert len('\ufdd0d800') == 5
assert len('\ufdefd800') == 5
assert '\N{LATIN CAPITAL LETTER A}' == 'A'
assert int('\N{EM SPACE}12\N{EN SPACE}') == 12
assert len('\ud800' 'a') == 2
assert '\ud800'.isdigit() is False
assert '\ud800'.isprintable() is False
assert '\ud800'.isascii() is False
match '\ud800':
    case '\ud800':
        print('surrogate pattern matched')
    case _:
        raise AssertionError('surrogate pattern lost')
print('named escapes and literal noncharacters preserved')
