// The Lumen library file langs/lib_lumen/to_string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function frac_to_base_string($f, $radix, $limit) {
    $alphabet = "0123456789abcdefghijklmnopqrstuvwxyz";
    $result = "";
    $count_ = 0;
    while ($count_ < $limit) {
        if ($f == floatval(0)) {
            return $result;
        }
        $f = $f * floatval($radix);
        $digit = intval($f);
        $result = $result . $alphabet[$digit];
        $f = frac($f);
        $count_ = $count_ + 1;
    }
    return $result;
}
