import json
print(json.dumps([1.0, -0.0]))
print(json.dumps(json.loads('[NaN, Infinity, -Infinity, -0.0]')))
print(json.dumps({1.0: 'x'}))
try:
    json.dumps([], indent=1.5)
except:
    print('bad indent')
try:
    json.dumps(json.loads('NaN'), allow_nan=False)
except:
    print('finite only')
