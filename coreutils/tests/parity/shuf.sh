#!/usr/bin/env bash
# coreutils/tests/parity/shuf.sh
echo "=== shuf ==="

# Note: shuf is randomized. We can't match exact output.
# But we can check if it produces the same number of lines and contains the same elements.
# To test parity accurately, we'd need to set a random seed if possible.
# Corvo math.random doesn't support seeds yet.
# We'll just check if it exits successfully and respects -n.

run_case shuf "count -n 5"         "gnu-shuf -n 5 /fixtures/long.txt | wc -l"         "corvo /corvo/coreutils/shuf.corvo -- -n 5 /fixtures/long.txt | wc -l"
run_case shuf "stdin"             "seq 10 | gnu-shuf | sort -n"                     "seq 10 | corvo /corvo/coreutils/shuf.corvo | sort -n"
