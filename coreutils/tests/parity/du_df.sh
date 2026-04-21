#!/usr/bin/env bash
# coreutils/tests/parity/du_df.sh
echo "=== du / df ==="

run_case du "plain"               "gnu-du /fixtures/a.txt"                       "corvo /corvo/coreutils/du.corvo -- /fixtures/a.txt"
run_case du "summary"             "gnu-du -s /fixtures"                         "corvo /corvo/coreutils/du.corvo -- -s /fixtures"
run_case df "plain"               "gnu-df /fixtures"                            "corvo /corvo/coreutils/df.corvo -- /fixtures"
