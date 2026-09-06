#!/bin/bash
# Round-trip benchmark for the microcode2 kernel's emitter: every example,
# in every language, written out in every language by `--emit`, run on the
# stack kernel, and compared with the original's output. Prints a table
# per target language: programs that round-trip, programs the target
# cannot spell (with the reason counted in translate_skips.txt), and
# programs that wrote but printed something else.
cd "$(dirname "$0")/.." || exit 1
cargo build --quiet || exit 1
B=./target/debug/lumen-lang
T=${TMPDIR:-/tmp}/lumen-translate
mkdir -p "$T"
flag_for() {
    case "$1" in
        php) echo "--lang langs/extras/php.json" ;; rb) echo "--lang langs/extras/ruby.json" ;;
        pas) echo "--lang langs/extras/pascal.json" ;; c) echo "--lang langs/extras/c.json" ;;
        js) echo "--lang langs/extras/javascript.json" ;; swift) echo "--lang langs/extras/swift.json" ;;
    esac
}
TARGETS="lm rpl py rs c js pas php rb swift"
declare -A OK SKIP BAD
for t in $TARGETS; do OK[$t]=0; SKIP[$t]=0; BAD[$t]=0; done
: > "$T/skips.txt"; : > "$T/wrong.txt"
for f in $(find examples -type f ! -name "*.md" | sort); do
    ext="${f##*.}"
    src=$(flag_for "$ext")
    # shellcheck disable=SC2086
    expected=$(timeout 30 $B --kernel stack $src "$f" 2>&1)
    for t in $TARGETS; do
        tflag=$(flag_for "$t")
        target="${tflag#--lang }"; [ -z "$target" ] && target="$t"
        out="$T/program.$t"
        # shellcheck disable=SC2086
        if ! timeout 30 $B --kernel microcode2 $src --emit "$target" "$f" > "$out" 2> "$T/error.txt"; then
            SKIP[$t]=$((SKIP[$t] + 1)); echo "$f -> $t: $(head -c 200 "$T/error.txt")" >> "$T/skips.txt"; continue
        fi
        # shellcheck disable=SC2086
        got=$(timeout 30 $B --kernel stack $tflag "$out" 2>&1)
        if [ "$got" = "$expected" ]; then
            OK[$t]=$((OK[$t] + 1))
        else
            BAD[$t]=$((BAD[$t] + 1)); echo "== $f -> $t" >> "$T/wrong.txt"; diff <(echo "$expected") <(echo "$got") | head -4 >> "$T/wrong.txt"
        fi
    done
done
echo "target   round-trips  unwritable  wrong"
total_ok=0; total_bad=0
for t in $TARGETS; do
    printf "%-8s %11d %11d %6d\n" "$t" "${OK[$t]}" "${SKIP[$t]}" "${BAD[$t]}"
    total_ok=$((total_ok + OK[$t])); total_bad=$((total_bad + BAD[$t]))
done
echo "round-trips: $total_ok, wrong: $total_bad (reasons in $T/skips.txt, differences in $T/wrong.txt)"
exit $total_bad
