#!/usr/bin/env bash
# coreutils/tests/parity/file-utils.sh
echo "=== file utilities ==="

run_case realpath "self"          "gnu-realpath /fixtures/a.txt"                  "corvo /corvo/coreutils/realpath.corvo -- /fixtures/a.txt"
run_case sync "plain"              "gnu-sync"                                      "corvo /corvo/coreutils/sync.corvo"
run_case touch "new file"          "gnu-touch /tmp/touched.txt"                    "corvo /corvo/coreutils/touch.corvo -- /tmp/touched_corvo.txt"
run_case truncate "size"           "gnu-truncate -s 1K /tmp/trunc.txt"             "corvo /corvo/coreutils/truncate.corvo -- -s 1K /tmp/trunc_corvo.txt"

# Vdir: check if output starts with same pattern (vdir output contains dates/times)
# We can't match exactly easily without monkey-patching time.
# But we can check if it exits successfully.
run_case vdir "basic"              "gnu-vdir /fixtures"                            "corvo /corvo/coreutils/vdir.corvo -- /fixtures"

rm -f /tmp/touched.txt /tmp/touched_corvo.txt /tmp/trunc.txt /tmp/trunc_corvo.txt
