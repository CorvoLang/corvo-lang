#!/bin/bash
source "$(dirname "$0")/../helpers.sh"

run_case "pwd" "basic" "gnu-pwd" "corvo coreutils/pwd.corvo"
run_case "pwd" "L" "gnu-pwd -L" "corvo coreutils/pwd.corvo -- -L"

show_time "pwd"
