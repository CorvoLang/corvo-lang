#!/usr/bin/env bash
# coreutils/tests/parity/sha1sum.sh
echo "=== sha1sum ==="
cd /fixtures

run_case sha1sum "single file"     "gnu-sha1sum a.txt"        "corvo /corvo/coreutils/sha1sum.corvo -- a.txt"
run_case sha1sum "multiple files"   "gnu-sha1sum a.txt b.txt"  "corvo /corvo/coreutils/sha1sum.corvo -- a.txt b.txt"
run_case sha1sum "stdin"           "cat a.txt | gnu-sha1sum"   "cat a.txt | corvo /corvo/coreutils/sha1sum.corvo"

run_uutils_case sha1sum "single file" "uu-sha1sum a.txt"      "corvo /corvo/coreutils/sha1sum.corvo -- a.txt"
