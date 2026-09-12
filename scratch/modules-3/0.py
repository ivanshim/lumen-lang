import json
print(json.dumps({"a": [1, 2.5, None, True]}, sort_keys=True), json.dumps(json.loads('{"k": [1, "x"]}')))
