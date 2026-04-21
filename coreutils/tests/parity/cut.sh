#!/usr/bin/env bash
# coreutils/tests/parity/cut.sh
# Required parity cases for cut.
# Sourced by parity/run-all.sh; requires run_case, run_uutils_case, show_time
# and the PASS / FAIL counters to be in scope.

echo "=== cut ==="
cd /fixtures

# Create test files
echo -e "a:b:c:d\n1:2:3:4\nw:x:y:z" > /tmp/cut_colon.txt
echo -e "field1\tfield2\tfield3\n1\t2\t3" > /tmp/cut_tab.txt
echo -e "abcdefgh\nijklmnop" > /tmp/cut_chars.txt

run_case cut "field 2"                "gnu-cut -d: -f2 /tmp/cut_colon.txt"            "corvo /corvo/coreutils/cut.corvo -- -d: -f2 /tmp/cut_colon.txt"
run_case cut "fields 1,3"             "gnu-cut -d: -f1,3 /tmp/cut_colon.txt"          "corvo /corvo/coreutils/cut.corvo -- -d: -f1,3 /tmp/cut_colon.txt"
run_case cut "field range 2-3"        "gnu-cut -d: -f2-3 /tmp/cut_colon.txt"          "corvo /corvo/coreutils/cut.corvo -- -d: -f2-3 /tmp/cut_colon.txt"
run_case cut "tab delimiter"          "gnu-cut -f2 /tmp/cut_tab.txt"                  "corvo /corvo/coreutils/cut.corvo -- -f2 /tmp/cut_tab.txt"
run_case cut "characters 1-4"         "gnu-cut -c1-4 /tmp/cut_chars.txt"              "corvo /corvo/coreutils/cut.corvo -- -c1-4 /tmp/cut_chars.txt"
run_case cut "bytes 2-5"              "gnu-cut -b2-5 /tmp/cut_chars.txt"              "corvo /corvo/coreutils/cut.corvo -- -b2-5 /tmp/cut_chars.txt"
run_case cut "field from end 2-"      "gnu-cut -d: -f2- /tmp/cut_colon.txt"           "corvo /corvo/coreutils/cut.corvo -- -d: -f2- /tmp/cut_colon.txt"
run_case cut "stdin"                   "echo 'a:b:c' | gnu-cut -d: -f2"               "echo 'a:b:c' | corvo /corvo/coreutils/cut.corvo -- -d: -f2"
run_case cut "stdin dash"              "echo 'a:b:c' | gnu-cut -d: -f2 -"             "echo 'a:b:c' | corvo /corvo/coreutils/cut.corvo -- -d: -f2 -"

run_uutils_case cut "field 2"         "uu-cut -d: -f2 /tmp/cut_colon.txt"             "corvo /corvo/coreutils/cut.corvo -- -d: -f2 /tmp/cut_colon.txt"

show_time "gnu-cut"   gnu-cut -d: -f2 /tmp/cut_colon.txt
show_time "corvo cut" corvo /corvo/coreutils/cut.corvo -- -d: -f2 /tmp/cut_colon.txt

# Cleanup
rm -f /tmp/cut_colon.txt /tmp/cut_tab.txt /tmp/cut_chars.txt
