#!/usr/bin/env bash
# coreutils/tests/parity/basenc.sh
# Required parity cases for basenc.
# Sourced by parity/run-all.sh; requires run_case, run_uutils_case, show_time
# and the PASS / FAIL counters to be in scope.

echo "=== basenc ==="

echo -n "Hello!" > /tmp/basenc_a.txt
printf "SGVsbG8h" > /tmp/basenc_b64.txt

run_case basenc "--base64 encode"     "gnu-basenc --base64 /tmp/basenc_a.txt"              "corvo /corvo/coreutils/basenc.corvo -- --base64 /tmp/basenc_a.txt"
run_case basenc "--base64 decode"     "gnu-basenc --base64 -d /tmp/basenc_b64.txt"         "corvo /corvo/coreutils/basenc.corvo -- --base64 -d /tmp/basenc_b64.txt"
run_case basenc "--base32 encode"     "gnu-basenc --base32 /tmp/basenc_a.txt"              "corvo /corvo/coreutils/basenc.corvo -- --base32 /tmp/basenc_a.txt"
run_case basenc "--base16 encode"     "gnu-basenc --base16 /tmp/basenc_a.txt"              "corvo /corvo/coreutils/basenc.corvo -- --base16 /tmp/basenc_a.txt"
run_case basenc "missing encoding"    "gnu-basenc /tmp/basenc_a.txt"                       "corvo /corvo/coreutils/basenc.corvo -- /tmp/basenc_a.txt"
run_case basenc "missing file"        "gnu-basenc --base64 /tmp/basenc_no_such"            "corvo /corvo/coreutils/basenc.corvo -- --base64 /tmp/basenc_no_such"
run_case basenc "stdin"               "echo -n 'hello' | gnu-basenc --base64"             "echo -n 'hello' | corvo /corvo/coreutils/basenc.corvo -- --base64"
run_case basenc "stdin dash"          "echo -n 'hello' | gnu-basenc --base64 -"           "echo -n 'hello' | corvo /corvo/coreutils/basenc.corvo -- --base64 -"

show_time "gnu-basenc"   gnu-basenc --base64 /tmp/basenc_a.txt
show_time "corvo basenc" corvo /corvo/coreutils/basenc.corvo -- --base64 /tmp/basenc_a.txt

# Cleanup
rm -f /tmp/basenc_a.txt /tmp/basenc_b64.txt
