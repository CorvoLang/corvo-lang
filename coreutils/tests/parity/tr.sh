#!/usr/bin/env bash
# coreutils/tests/parity/tr.sh
echo "=== tr ==="

run_case tr "lower to upper"     "echo 'hello' | gnu-tr 'a-z' 'A-Z'"  "echo 'hello' | corvo /corvo/coreutils/tr.corvo -- 'a-z' 'A-Z'"
run_case tr "delete digits"      "echo 'h3ll0' | gnu-tr -d '0-9'"    "echo 'h3ll0' | corvo /corvo/coreutils/tr.corvo -- -d '0-9'"
run_case tr "squeeze"            "echo 'aaabbbccc' | gnu-tr -s 'abc'" "echo 'aaabbbccc' | corvo /corvo/coreutils/tr.corvo -- -s 'abc'"
