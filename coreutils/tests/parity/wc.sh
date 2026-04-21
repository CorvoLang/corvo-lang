#!/usr/bin/env bash
# coreutils/tests/parity/wc.sh
# Required parity cases for wc.
# Sourced by parity/run-all.sh; requires run_case, run_uutils_case, show_time
# and the PASS / FAIL counters to be in scope.

echo "=== wc ==="
cd /fixtures

# Create test files
echo -e "hello world\ntest line two\nthird line" > /tmp/wc_test.txt
echo -e "one\ntwo\nthree\nfour\nfive" > /tmp/wc_lines.txt
echo -n "no newline at end" > /tmp/wc_nonl.txt

run_case wc "basic count"             "gnu-wc /tmp/wc_test.txt"                       "corvo /corvo/coreutils/wc.corvo -- /tmp/wc_test.txt"
run_case wc "lines only"              "gnu-wc -l /tmp/wc_test.txt"                    "corvo /corvo/coreutils/wc.corvo -- -l /tmp/wc_test.txt"
run_case wc "words only"              "gnu-wc -w /tmp/wc_test.txt"                    "corvo /corvo/coreutils/wc.corvo -- -w /tmp/wc_test.txt"
run_case wc "bytes only"              "gnu-wc -c /tmp/wc_test.txt"                    "corvo /corvo/coreutils/wc.corvo -- -c /tmp/wc_test.txt"
run_case wc "multiple files"          "gnu-wc /tmp/wc_test.txt /tmp/wc_lines.txt"     "corvo /corvo/coreutils/wc.corvo -- /tmp/wc_test.txt /tmp/wc_lines.txt"
run_case wc "no trailing newline"     "gnu-wc /tmp/wc_nonl.txt"                       "corvo /corvo/coreutils/wc.corvo -- /tmp/wc_nonl.txt"
run_case wc "lines + words"           "gnu-wc -lw /tmp/wc_test.txt"                   "corvo /corvo/coreutils/wc.corvo -- -l -w /tmp/wc_test.txt"
run_case wc "stdin"                   "echo 'abc\ndef' | gnu-wc"                      "echo 'abc\ndef' | corvo /corvo/coreutils/wc.corvo"
run_case wc "stdin dash"              "echo 'abc\ndef' | gnu-wc -"                    "echo 'abc\ndef' | corvo /corvo/coreutils/wc.corvo -- -"

run_uutils_case wc "basic count"      "uu-wc /tmp/wc_test.txt"                        "corvo /corvo/coreutils/wc.corvo -- /tmp/wc_test.txt"

show_time "gnu-wc"    gnu-wc /tmp/wc_test.txt
show_time "corvo wc"  corvo /corvo/coreutils/wc.corvo -- /tmp/wc_test.txt

# Cleanup
rm -f /tmp/wc_test.txt /tmp/wc_lines.txt /tmp/wc_nonl.txt
