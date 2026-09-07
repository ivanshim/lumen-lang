#!/bin/bash
# Differential test: every example must print the same thing on every kernel.
# The check lives in test.sh now: a program runs on stream35 first and every
# other kernel must print what it printed, so one pass over the examples does
# both jobs. This runs that pass over every language.
cd "$(dirname "$0")/.." || exit 1
exec ./test.sh --lang all "$@"
