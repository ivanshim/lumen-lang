// The Lumen library file langs/lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

fn repeat_string(s: i64, repetitions: i64) -> String {
    let mut out = "";
    let mut i = 0;
    while i < repetitions {
        out = out + s;
        i = i + 1;
    }
    return out;
}

fn join_strings(arr: i64, separator: i64) -> String {
    let mut out = "";
    let n = arr.len();
    let mut i = 0;
    while i < n {
        if i > 0 {
            out = out + separator;
        }
        out = out + arr[i];
        i = i + 1;
    }
    return out;
}
