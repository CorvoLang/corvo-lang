#!/usr/bin/env bash
# coreutils/tests/parity/comm.sh
echo "=== comm ==="

printf "a\nb\nc\n" > /tmp/comm_a.txt
printf "b\nc\nd\n" > /tmp/comm_b.txt

run_case comm "plain"            "gnu-comm /tmp/comm_a.txt /tmp/comm_b.txt"      "corvo /corvo/coreutils/comm.corvo -- /tmp/comm_a.txt /tmp/comm_b.txt"
run_case comm "suppress 1"       "gnu-comm -1 /tmp/comm_a.txt /tmp/comm_b.txt"   "corvo /corvo/coreutils/comm.corvo -- -1 /tmp/comm_a.txt /tmp/comm_b.txt"
run_case comm "suppress 2"       "gnu-comm -2 /tmp/comm_a.txt /tmp/comm_b.txt"   "corvo /corvo/coreutils/comm.corvo -- -2 /tmp/comm_a.txt /tmp/comm_b.txt"
run_case comm "suppress 3"       "gnu-comm -3 /tmp/comm_a.txt /tmp/comm_b.txt"   "corvo /corvo/coreutils/comm.corvo -- -3 /tmp/comm_a.txt /tmp/comm_b.txt"
run_case comm "stdin first"      "printf 'a\nb\n' | gnu-comm - /tmp/comm_b.txt"  "printf 'a\nb\n' | corvo /corvo/coreutils/comm.corvo -- - /tmp/comm_b.txt"
run_case comm "stdin second"     "printf 'b\nc\n' | gnu-comm /tmp/comm_a.txt -"  "printf 'b\nc\n' | corvo /corvo/coreutils/comm.corvo -- /tmp/comm_a.txt -"

rm -f /tmp/comm_a.txt /tmp/comm_b.txt
