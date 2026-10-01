#!/bin/bash
# Differential test: selected kernels must exit 0 and print the same output.
# Python runs on stack8 and microcode7 because reference kernels ignore ext.*
# arithmetic labels. Other languages use all six kernels against stream35.
# test.sh owns this rule; run its pass over every language.
cd "$(dirname "$0")/.." || exit 1
exec ./test.sh --lang all "$@"
