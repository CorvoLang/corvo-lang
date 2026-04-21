#!/usr/bin/env bash
# coreutils/tests/parity/printf.sh
echo "=== printf ==="

run_case printf "plain"           "gnu-printf 'hello\n'"                          "corvo /corvo/coreutils/printf.corvo -- 'hello\n'"
run_case printf "args"            "gnu-printf 'val: %s\n' 'foo'"                 "corvo /corvo/coreutils/printf.corvo -- 'val: %s\n' 'foo'"
run_case printf "number"          "gnu-printf 'num: %d\n' 42"                   "corvo /corvo/coreutils/printf.corvo -- 'num: %d\n' 42"
