#!/usr/bin/env bash
# coreutils/tests/parity/sha512sum.sh
echo "=== sha512sum ==="
cd /fixtures

run_case sha512sum "single file"     "gnu-sha512sum a.txt"        "corvo /corvo/coreutils/sha512sum.corvo -- a.txt"
run_case sha512sum "multiple files"   "gnu-sha512sum a.txt b.txt"  "corvo /corvo/coreutils/sha512sum.corvo -- a.txt b.txt"
run_case sha512sum "stdin"           "cat a.txt | gnu-sha512sum"   "cat a.txt | corvo /corvo/coreutils/sha512sum.corvo"

run_uutils_case sha512sum "single file" "uu-sha512sum a.txt"      "corvo /corvo/coreutils/sha512sum.corvo -- a.txt"
