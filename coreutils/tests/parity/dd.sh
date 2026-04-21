#!/usr/bin/env bash
# coreutils/tests/parity/dd.sh
echo "=== dd ==="

# Create a source file
printf "0123456789ABCDEF" > /tmp/dd_in.txt

run_case dd "basic copy"          "gnu-dd if=/tmp/dd_in.txt of=/tmp/dd_out.txt status=none && cat /tmp/dd_out.txt" "corvo /corvo/coreutils/dd.corvo -- if=/tmp/dd_in.txt of=/tmp/dd_out_corvo.txt status=none && cat /tmp/dd_out_corvo.txt"
run_case dd "count & bs"          "gnu-dd if=/tmp/dd_in.txt bs=2 count=4 status=none" "corvo /corvo/coreutils/dd.corvo -- if=/tmp/dd_in.txt bs=2 count=4 status=none"
run_case dd "skip"                "gnu-dd if=/tmp/dd_in.txt bs=1 skip=5 status=none" "corvo /corvo/coreutils/dd.corvo -- if=/tmp/dd_in.txt bs=1 skip=5 status=none"

rm -f /tmp/dd_in.txt /tmp/dd_out.txt /tmp/dd_out_corvo.txt
