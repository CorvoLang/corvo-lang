#!/usr/bin/env bash
# coreutils/tests/parity/md5sum.sh
echo "=== md5sum ==="
cd /fixtures

run_case md5sum "single file"     "gnu-md5sum a.txt"        "corvo /corvo/coreutils/md5sum.corvo -- a.txt"
run_case md5sum "multiple files"   "gnu-md5sum a.txt b.txt"  "corvo /corvo/coreutils/md5sum.corvo -- a.txt b.txt"
run_case md5sum "stdin"           "cat a.txt | gnu-md5sum"   "cat a.txt | corvo /corvo/coreutils/md5sum.corvo"

run_uutils_case md5sum "single file" "uu-md5sum a.txt"      "corvo /corvo/coreutils/md5sum.corvo -- a.txt"
