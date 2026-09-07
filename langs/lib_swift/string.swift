// The Lumen library file langs/lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

func repeat_string(s: Int, repetitions: Int) -> String {
    var out = ""
    var i = 0
    while i < repetitions {
        out = out + s
        i = i + 1
    }
    return out
}

func join_strings(arr: Int, separator: Int) -> String {
    var out = ""
    let n = arr.count
    var i = 0
    while i < n {
        if i > 0 {
            out = out + separator
        }
        out = out + arr[i]
        i = i + 1
    }
    return out
}
