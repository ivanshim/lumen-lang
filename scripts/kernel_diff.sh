#!/bin/bash
# Differential test: every example must print the same thing on both kernels.
# test.sh checks that programs run; this checks that the two independent
# implementations agree on what they print. Exit status is the number of
# programs whose output differs.
cd "$(dirname "$0")/.." || exit 1
cargo build --quiet || exit 1
B=./target/debug/lumen-lang
same=0; differing=0
# Languages in langs/extras/ are passed as a definition file.
flag_for() {
    case "${1##*.}" in
        php) echo "--lang langs/extras/php.json" ;;
        rb) echo "--lang langs/extras/ruby.json" ;;
        pas) echo "--lang langs/extras/pascal.json" ;;
        c) echo "--lang langs/extras/c.json" ;;
        js) echo "--lang langs/extras/javascript.json" ;;
        swift) echo "--lang langs/extras/swift.json" ;;
    esac
}
for f in $(find examples -type f \( -name "*.lm" -o -name "*.rpl" -o -name "*.py" -o -name "*.rs" -o -name "*.php" -o -name "*.rb" -o -name "*.pas" -o -name "*.c" -o -name "*.js" -o -name "*.swift" \) | sort); do
    flag=$(flag_for "$f")
    # shellcheck disable=SC2086
    a=$($B --kernel stream $flag "$f" 2>&1)
    # shellcheck disable=SC2086
    b=$($B --kernel microcode $flag "$f" 2>&1)
    if [ "$a" = "$b" ]; then
        same=$((same + 1))
    else
        differing=$((differing + 1))
        echo "== $f"
        diff <(echo "$a") <(echo "$b") | head -8
    fi
done
echo "identical output: $same, differing: $differing"
exit $differing
