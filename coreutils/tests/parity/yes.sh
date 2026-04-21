#!/usr/bin/env bash
# coreutils/tests/parity/yes.sh
echo "=== yes ==="

# We MUST pipe to head because yes is infinite.
# The timeout in helpers.sh would catch it, but it's better to be explicit.
run_case yes "basic y"            "gnu-yes | head -n 5"          "corvo /corvo/coreutils/yes.corvo | head -n 5"
run_case yes "custom string"      "gnu-yes 'hello' | head -n 3"  "corvo /corvo/coreutils/yes.corvo -- 'hello' | head -n 3"
run_case yes "multiple args"      "gnu-yes 'a' 'b' | head -n 3"  "corvo /corvo/coreutils/yes.corvo -- 'a' 'b' | head -n 3"
