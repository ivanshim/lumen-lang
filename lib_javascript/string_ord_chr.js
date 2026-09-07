// The Lumen library file lib_lumen/string_ord_chr.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function reverse_characters(s) {
    let result = "";
    let index = s.length - 1;
    while (index >= 0) {
        result = result + s.charAt(index);
        index = index - 1;
    }
    return result;
}
