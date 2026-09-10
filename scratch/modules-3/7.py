import json
from os.path import join, exists, isfile, isdir
import os
import platform
value = json.loads('{"k": [1, "x"], "v": -2.5e2, "s": "\\u00e9"}')
print(value['k'][0], value['k'][1], value['v'], value['s'])
print(join('a', '/b', 'c'))
print(exists('langs/python.json'), isfile('langs/python.json'), isdir('langs'))
print('PATH' in os.environ, len(os.getcwd()) > 0, len(platform.system()) > 0, len(platform.machine()) > 0)
class Box:
    def __init__(self, value):
        self.value = value

def convert(value):
    return value.value

print(json.dumps(Box(7), default=convert))
