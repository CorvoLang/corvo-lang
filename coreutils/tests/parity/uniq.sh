#!/usr/bin/env bash
# coreutils/tests/parity/uniq.sh
echo "=== uniq ==="

printf "a\na\nb\nc\nc\nc\nd\n" > /tmp/uniq_input.txt

run_case uniq "plain"            "gnu-uniq /tmp/uniq_input.txt"      "corvo /corvo/coreutils/uniq.corvo -- /tmp/uniq_input.txt"
run_case uniq "stdin"            "cat /tmp/uniq_input.txt | gnu-uniq" "cat /tmp/uniq_input.txt | corvo /corvo/coreutils/uniq.corvo"
run_case uniq "stdin dash"       "cat /tmp/uniq_input.txt | gnu-uniq -" "cat /tmp/uniq_input.txt | corvo /corvo/coreutils/uniq.corvo -- -"

rm -f /tmp/uniq_input.txt
