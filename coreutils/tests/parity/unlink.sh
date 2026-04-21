#!/bin/bash
source "$(dirname "$0")/../helpers.sh"

echo "test" > test_unlink.txt
run_case "unlink" "basic" "gnu-unlink test_unlink.txt" "echo 'test' > test_unlink.txt && corvo coreutils/unlink.corvo -- test_unlink.txt"

show_time "unlink"
