#!/bin/bash
source "$(dirname "$0")/../helpers.sh"

run_case "seq" "last" "gnu-seq 5" "corvo coreutils/seq.corvo -- 5"
run_case "seq" "first-last" "gnu-seq 2 5" "corvo coreutils/seq.corvo -- 2 5"
run_case "seq" "first-incr-last" "gnu-seq 1 2 5" "corvo coreutils/seq.corvo -- 1 2 5"
run_case "seq" "negative" "gnu-seq 5 -1 1" "corvo coreutils/seq.corvo -- 5 -1 1"

show_time "seq"
