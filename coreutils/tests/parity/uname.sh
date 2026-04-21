#!/bin/bash
source "$(dirname "$0")/../helpers.sh"

run_case "uname" "basic" "gnu-uname" "corvo coreutils/uname.corvo"
run_case "uname" "all" "gnu-uname -a" "corvo coreutils/uname.corvo -- -a"

show_time "uname"
