// The Lumen library file langs/lib_lumen/to_string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function frac_to_base_string(f, radix, limit) {
    let digit;
    const alphabet = "0123456789abcdefghijklmnopqrstuvwxyz";
    let result = "";
    let count = 0;
    while (count < limit) {
        if (f === Number(0)) {
            return result;
        }
        f = f * Number(radix);
        digit = Math.trunc(f);
        result = result + alphabet.charAt(digit);
        f = frac(f);
        count = count + 1;
    }
    return result;
}
