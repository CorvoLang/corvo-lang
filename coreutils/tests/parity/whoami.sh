#!/bin/bash
source "$(dirname "$0")/../helpers.sh"

run_case "whoami" "basic" "gnu-whoami" "corvo coreutils/whoami.corvo"

show_time "whoami"
