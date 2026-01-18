# Stream Kernel Bug: Complex Loops with string_to_value

## Summary
The stream kernel has a complex bug when calling `string_to_value` in certain loop scenarios. The bug only manifests in complex multi-function cases with nested loops and function calls.

## Investigation Summary

### Simple Cases: WORK ✅
```lumen
// Simple loop with string_to_value - WORKS
let i = 0
let val = 0
while i < 5
    val = string_to_value("99")
    print("i=" . str(i))
    i = i + 1
```

### Complex Cases: FAIL ❌
The `test_string_comprehensive.lm` file times out on stream kernel but passes on microcode kernel. The file contains:
- Multiple functions with nested while loops
- Functions calling other functions
- `string_to_value` called inside nested loops within functions

### Root Cause Analysis

**Attempted Fix #1: Remove per-iteration scopes**
- Removed `env.push_scope()` / `env.pop_scope()` from while loops
- Result: Made it WORSE - loop counter got stuck at iteration 2
- Conclusion: Per-iteration scopes are necessary for correct semantics

**Discovery:**
The bug appears to be related to complex interactions between:
1. Per-iteration scope creation in while loops
2. Nested function calls
3. Multiple levels of scope push/pop operations
4. Calling Lumen library functions (string_to_value) from within loops

**Isolation Difficulty:**
- Cannot reproduce with simple test cases
- Only fails in complex multi-function scenarios
- Suggests subtle environment/scope corruption issue
- Not a straightforward logic error

##Impact
- **Microcode kernel**: All tests pass ✅ (82/82)
- **Stream kernel**: 81/82 tests pass (test_string_comprehensive.lm times out)
- Simple uses of `string_to_value` work fine
- Complex nested scenarios fail

## Workarounds

### Option 1: Document as Known Limitation
- Keep current state
- Document that `test_string_comprehensive.lm` is known to fail on stream kernel
- Note that simpler uses work fine
- Recommend microcode kernel for complex programs

### Option 2: Add parse_int Back
- Re-add `parse_int` as deprecated helper
- Use for simple integer parsing where `string_to_value` would be called in loops
- Maintain stream kernel compatibility

### Option 3: Skip Test on Stream Kernel
- Modify test suite to skip test_string_comprehensive.lm for stream kernel only
- Accept 81/82 pass rate for stream kernel

### Option 4: Deep Investigation Required
This requires:
- Detailed analysis of scope/environment state across iterations
- Profiling of scope creation/destruction
- Understanding interaction between Lumen library code and kernel
- Potentially redesigning scope management in stream kernel

## Recommendation
**Option 1** - Document as known limitation. The microcode kernel works perfectly and is the recommended kernel. The stream kernel is experimental and this complex edge case is acceptable given that:
- Simple cases all work
- The bug is isolated to complex multi-function scenarios
- Microcode kernel is authoritative and works correctly
