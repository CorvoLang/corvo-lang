#!/usr/bin/env bash
# coreutils/tests/parity/sha384sum.sh
echo "=== sha384sum ==="
cd /fixtures

run_case sha384sum "single file"     "gnu-sha384sum a.txt"        "corvo /corvo/coreutils/sha384sum.corvo -- a.txt"
run_case sha384sum "multiple files"   "gnu-sha384sum a.txt b.txt"  "corvo /corvo/coreutils/sha384sum.corvo -- a.txt b.txt"
run_case sha384sum "stdin"           "cat a.txt | gnu-sha384sum"   "cat a.txt | corvo /corvo/coreutils/sha384sum.corvo"

run_uutils_case sha384sum "single file" "uu-sha384sum a.txt"      "corvo /corvo/coreutils/sha384sum.corvo -- a.txt"
