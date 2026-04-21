#!/usr/bin/env bash
# coreutils/tests/parity/cksum.sh
# Required parity cases for cksum.
# Sourced by parity/run-all.sh; requires run_case, run_uutils_case, show_time
# and the PASS / FAIL counters to be in scope.

echo "=== cksum ==="

echo "hello world" > /tmp/cksum_a.txt
echo "test data" > /tmp/cksum_b.txt

run_case cksum "single file"          "gnu-cksum /tmp/cksum_a.txt"                         "corvo /corvo/coreutils/cksum.corvo -- /tmp/cksum_a.txt"
run_case cksum "two files"            "gnu-cksum /tmp/cksum_a.txt /tmp/cksum_b.txt"        "corvo /corvo/coreutils/cksum.corvo -- /tmp/cksum_a.txt /tmp/cksum_b.txt"
run_case cksum "missing file"         "gnu-cksum /tmp/cksum_no_such"                       "corvo /corvo/coreutils/cksum.corvo -- /tmp/cksum_no_such"
run_case cksum "stdin"                "echo -n 'hello' | gnu-cksum"                       "echo -n 'hello' | corvo /corvo/coreutils/cksum.corvo"
run_case cksum "stdin dash"           "echo -n 'hello' | gnu-cksum -"                     "echo -n 'hello' | corvo /corvo/coreutils/cksum.corvo -- -"

run_uutils_case cksum "single file"   "uu-cksum /tmp/cksum_a.txt"                          "corvo /corvo/coreutils/cksum.corvo -- /tmp/cksum_a.txt"

show_time "gnu-cksum"   gnu-cksum /tmp/cksum_a.txt
show_time "corvo cksum" corvo /corvo/coreutils/cksum.corvo -- /tmp/cksum_a.txt

# Cleanup
rm -f /tmp/cksum_a.txt /tmp/cksum_b.txt
