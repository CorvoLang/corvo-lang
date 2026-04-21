#!/usr/bin/env bash
# coreutils/tests/parity/sys-utils.sh
echo "=== system utilities ==="

run_case id "plain"                "gnu-id"                                        "corvo /corvo/coreutils/id.corvo"
run_case whoami "plain"            "gnu-whoami"                                    "corvo /corvo/coreutils/whoami.corvo"
run_case nproc "plain"             "gnu-nproc"                                     "corvo /corvo/coreutils/nproc.corvo"
run_case logname "plain"           "gnu-logname"                                   "corvo /corvo/coreutils/logname.corvo"
run_case sleep "short"             "gnu-sleep 0.1"                                 "corvo /corvo/coreutils/sleep.corvo -- 0.1"
run_case stat "file"               "gnu-stat /fixtures/a.txt"                      "corvo /corvo/coreutils/stat.corvo -- /fixtures/a.txt"
run_case stat "dir"                "gnu-stat /fixtures"                            "corvo /corvo/coreutils/stat.corvo -- /fixtures"
