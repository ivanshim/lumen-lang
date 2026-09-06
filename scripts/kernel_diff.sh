#!/bin/bash
# Differential test: every example must print the same thing on both kernels.
# test.sh checks that programs run; this checks that the two independent
# implementations agree on what they print. Exit status is the number of
# programs whose output differs.
cd "$(dirname "$0")/.." || exit 1
cargo build --quiet || exit 1
B=./target/debug/lumen-lang
same=0; differing=0
for f in examples/lumen/*.lm examples/lumen/constructs/*.lm examples/lumen/libraries/*.lm examples/python/*.py examples/rust/*.rs examples/php/*.php; do
    a=$($B --kernel stream "$f" 2>&1)
    b=$($B --kernel microcode "$f" 2>&1)
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
