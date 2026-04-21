#!/usr/bin/env bash
# coreutils/tests/parity/sort.sh
echo "=== sort ==="

run_case sort "plain a.txt"       "gnu-sort /fixtures/a.txt"          "corvo /corvo/coreutils/sort.corvo -- /fixtures/a.txt"
run_case sort "plain b.txt"       "gnu-sort /fixtures/b.txt"          "corvo /corvo/coreutils/sort.corvo -- /fixtures/b.txt"
run_case sort "stdin"             "echo 'z\na\nb' | gnu-sort"        "echo 'z\na\nb' | corvo /corvo/coreutils/sort.corvo"
run_case sort "stdin dash"        "echo 'z\na\nb' | gnu-sort -"      "echo 'z\na\nb' | corvo /corvo/coreutils/sort.corvo -- -"
run_case sort "mixed files"       "gnu-sort /fixtures/a.txt /fixtures/b.txt" "corvo /corvo/coreutils/sort.corvo -- /fixtures/a.txt /fixtures/b.txt"
