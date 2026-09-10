import json
from io import StringIO
print(json.dumps({'z': 'é', 'a': [False, None]}, ensure_ascii=True, sort_keys=True, separators=(',', ':')))
print(json.dumps([1, 2], indent=2))
stream = StringIO()
json.dump({'x': 3}, stream)
stream.seek(0)
print(json.dumps(json.load(stream)))
try:
    json.loads('{bad}')
except json.JSONDecodeError:
    print('bad JSON')
