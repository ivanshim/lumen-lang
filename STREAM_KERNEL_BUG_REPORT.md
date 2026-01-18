# Stream Kernel Bug: string_to_value in Loops

## Summary
The stream kernel has a critical bug when calling `string_to_value` inside while loops. The loop counter malfunctions and either skips iterations or gets stuck in an infinite loop.

## Reproduction

```lumen
let i = 0
let val = 0
while i < 5
    val = string_to_value("123")
    print("i=" . str(i) . " val=" . str(val))
    i = i + 1
```

### Expected Output (Microcode Kernel)
```
i=0 val=123
i=1 val=123
i=2 val=123
i=3 val=123
i=4 val=123
```

### Actual Output (Stream Kernel)
```
i=0 val=123
i=4 val=123  (repeats infinitely)
```

## Root Cause
The stream kernel appears to have a bug in how it handles:
1. `let` declarations inside while loops (only runs last iteration)
2. Calling `string_to_value` inside while loops (corrupts loop counter)

## Impact
- `test_string_comprehensive.lm` times out on stream kernel
- `test_char_utilities.lm` works (simpler case)
- Microcode kernel: **All tests pass** ✅

## Workarounds

### Option 1: Keep parse_int for stream kernel compatibility
Re-add `parse_int` as a simple helper that doesn't trigger the stream kernel bug.

### Option 2: Skip failing test for stream kernel
Add `test_string_comprehensive.lm` to the stream kernel's skip list.

### Option 3: Fix stream kernel
This is a kernel-level bug that needs investigation in the stream kernel's execution logic.

## Recommendation
Until the stream kernel bug is fixed, keep `parse_int` as a workaround for simple integer parsing in loops. Mark it as deprecated but document that it's needed for stream kernel compatibility.
