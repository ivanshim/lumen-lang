def literal_methods(invalid):
    {}.update(**invalid)
    {1: 1, 2: 2}.keys()
    {1: 1, 2: 2}.items()
    list(reversed({}.values()))

print('read literal methods')
