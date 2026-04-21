#!/usr/bin/env bash
# coreutils/tests/parity/base64.sh
# Required parity cases for base64.
# Sourced by parity/run-all.sh; requires run_case, run_uutils_case, show_time
# and the PASS / FAIL counters to be in scope.

echo "=== base64 ==="

echo -n "Hello, World!" > /tmp/base64_a.txt
printf "SGVsbG8sIFdvcmxkIQ==" > /tmp/base64_encoded.txt

run_case base64 "encode file"         "gnu-base64 /tmp/base64_a.txt"                       "corvo /corvo/coreutils/base64.corvo -- /tmp/base64_a.txt"
run_case base64 "decode file"         "gnu-base64 -d /tmp/base64_encoded.txt"              "corvo /corvo/coreutils/base64.corvo -- -d /tmp/base64_encoded.txt"
run_case base64 "wrap=0 (no wrap)"    "gnu-base64 -w 0 /tmp/base64_a.txt"                  "corvo /corvo/coreutils/base64.corvo -- -w 0 /tmp/base64_a.txt"
run_case base64 "wrap=4"              "gnu-base64 -w 4 /tmp/base64_a.txt"                  "corvo /corvo/coreutils/base64.corvo -- -w 4 /tmp/base64_a.txt"
run_case base64 "missing file"        "gnu-base64 /tmp/base64_no_such"                     "corvo /corvo/coreutils/base64.corvo -- /tmp/base64_no_such"
run_case base64 "stdin"              "echo -n 'hello' | gnu-base64"                      "echo -n 'hello' | corvo /corvo/coreutils/base64.corvo"
run_case base64 "stdin dash"         "echo -n 'hello' | gnu-base64 -"                    "echo -n 'hello' | corvo /corvo/coreutils/base64.corvo -- -"

show_time "gnu-base64"   gnu-base64 /tmp/base64_a.txt
show_time "corvo base64" corvo /corvo/coreutils/base64.corvo -- /tmp/base64_a.txt

# Cleanup
rm -f /tmp/base64_a.txt /tmp/base64_encoded.txt
