#!/usr/bin/env bash
# coreutils/tests/parity/expand.sh
echo "=== expand / unexpand ==="

printf "a\tb\n" > /tmp/tabs.txt

run_case expand "basic"          "gnu-expand /tmp/tabs.txt"                      "corvo /corvo/coreutils/expand.corvo -- /tmp/tabs.txt"
run_case expand "stdin"          "printf 'x\ty\n' | gnu-expand"                 "printf 'x\ty\n' | corvo /corvo/coreutils/expand.corvo"
run_case unexpand "basic"        "gnu-unexpand -a /tmp/tabs.txt"                "corvo /corvo/coreutils/unexpand.corvo -- -a /tmp/tabs.txt"

rm -f /tmp/tabs.txt
