#!/bin/bash
# Time every program under bench/ on every kernel, best of N runs, as a
# markdown table. Release binaries. Usage: scripts/bench.sh [N] [kernel...]
cd "$(dirname "$0")/.." || exit 1
N=${1:-5}; shift
KERNELS=${*:-"microcode10 stack26 microcode11 microcode4 stack5 microlab stacklab"}
cargo build --release --quiet 2>/dev/null || { echo "build failed"; exit 1; }
TIMEFORMAT=%R
run() {  # kernel file
    case "$1" in
        stacklab|microlab) ./target/release/lumen-$1 "$2" ;;
        *) ./target/release/lumen-lang --kernel "$1" "$2" ;;
    esac
}
printf "| Program |"; for k in $KERNELS; do printf " %s |" "$k"; done; echo
printf "%s" "|---|"; for k in $KERNELS; do printf "%s" "---|"; done; echo
for f in bench/*.lm; do
    printf "| %s |" "$(basename "$f" .lm)"
    for k in $KERNELS; do
        best=999
        for i in $(seq "$N"); do
            t=$( { time run "$k" "$f" > /dev/null 2>&1; } 2>&1 )
            best=$(python3 -c "print(min($best, $t))")
        done
        printf " %.3f |" "$best"
    done
    echo
done
