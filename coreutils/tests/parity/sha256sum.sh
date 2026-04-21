#!/usr/bin/env bash
# coreutils/tests/parity/sha256sum.sh
echo "=== sha256sum ==="
cd /fixtures

run_case sha256sum "single file"     "gnu-sha256sum a.txt"        "corvo /corvo/coreutils/sha256sum.corvo -- a.txt"
run_case sha256sum "multiple files"   "gnu-sha256sum a.txt b.txt"  "corvo /corvo/coreutils/sha256sum.corvo -- a.txt b.txt"
run_case sha256sum "stdin"           "cat a.txt | gnu-sha256sum"   "cat a.txt | corvo /corvo/coreutils/sha256sum.corvo"

run_uutils_case sha256sum "single file" "uu-sha256sum a.txt"      "corvo /corvo/coreutils/sha256sum.corvo -- a.txt"
