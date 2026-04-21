#!/usr/bin/env bash
# coreutils/tests/parity/split.sh
echo "=== split / csplit ==="

# Create a source file
i=1; while [ $i -le 20 ]; do echo "line $i"; i=$((i+1)); done > /tmp/split_in.txt

# Test split
run_case split "by lines"         "gnu-split -l 5 /tmp/split_in.txt /tmp/s_gnu_ && ls /tmp/s_gnu_*" \
                                "corvo /corvo/coreutils/split.corvo -- -l 5 /tmp/split_in.txt /tmp/s_corvo_ && ls /tmp/s_corvo_*"

# Test csplit
run_case csplit "by pattern"      "gnu-csplit /tmp/split_in.txt '/line 10/' && ls xx*" \
                                "corvo /corvo/coreutils/csplit.corvo -- /tmp/split_in.txt '/line 10/' && ls xx*"

rm -f /tmp/split_in.txt /tmp/s_gnu_* /tmp/s_corvo_* xx*
