#!/usr/bin/env bash
# coreutils/tests/parity/join.sh
echo "=== join ==="

printf "1 a\n2 b\n" > /tmp/join_a.txt
printf "1 x\n2 y\n" > /tmp/join_b.txt

run_case join "plain"            "gnu-join /tmp/join_a.txt /tmp/join_b.txt"      "corvo /corvo/coreutils/join.corvo -- /tmp/join_a.txt /tmp/join_b.txt"
run_case join "stdin second"     "printf '1 x\n2 y\n' | gnu-join /tmp/join_a.txt -" "printf '1 x\n2 y\n' | corvo /corvo/coreutils/join.corvo -- /tmp/join_a.txt -"

rm -f /tmp/join_a.txt /tmp/join_b.txt
