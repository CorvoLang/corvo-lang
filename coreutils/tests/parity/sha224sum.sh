#!/usr/bin/env bash
# coreutils/tests/parity/sha224sum.sh
echo "=== sha224sum ==="
cd /fixtures

run_case sha224sum "single file"     "gnu-sha224sum a.txt"        "corvo /corvo/coreutils/sha224sum.corvo -- a.txt"
run_case sha224sum "multiple files"   "gnu-sha224sum a.txt b.txt"  "corvo /corvo/coreutils/sha224sum.corvo -- a.txt b.txt"
run_case sha224sum "stdin"           "cat a.txt | gnu-sha224sum"   "cat a.txt | corvo /corvo/coreutils/sha224sum.corvo"

run_uutils_case sha224sum "single file" "uu-sha224sum a.txt"      "corvo /corvo/coreutils/sha224sum.corvo -- a.txt"
