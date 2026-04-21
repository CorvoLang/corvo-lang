#!/bin/bash
source "$(dirname "$0")/../helpers.sh"

run_case "true" "basic" "gnu-true" "corvo coreutils/true.corvo"

show_time "true"
