#!/usr/bin/env bash
# coreutils/tests/helpers.sh
# Shared helper functions for all parity and matrix test scripts.
# Source this file; do NOT execute it directly.
#
# Callers must declare PASS and FAIL as integer variables before sourcing.

# Current command used for Corvo. Can be overridden set to a compiled binary path.
CORVO_BIN="corvo"

# Compare GNU vs Corvo: exit code and stdout must match.
# Args: section label gnu_cmd corvo_cmd
run_case() {
  local section="$1" label="$2" gnu_cmd="$3" corvo_cmd="$4"
  local gnu_ec=0 corvo_ec=0
  
  # 1. Run GNU (Reference)
  timeout 10s eval "$gnu_cmd"   > /tmp/t_gnu.out   2>/tmp/t_gnu.err   || gnu_ec=$?
  
  # 2. Run Corvo (Interpreted)
  timeout 10s eval "$corvo_cmd" > /tmp/t_corvo_int.out 2>/tmp/t_corvo_int.err || corvo_ec=$?
  
  # 3. Compare Interpreted
  if [[ "$gnu_ec" != "$corvo_ec" ]]; then
    printf "FAIL [%-4s] %-46s exit (int): gnu=%s corvo=%s\n" \
      "$section" "$label" "$gnu_ec" "$corvo_ec"
    FAIL=$((FAIL+1)); return
  fi
  if ! diff -q /tmp/t_gnu.out /tmp/t_corvo_int.out >/dev/null 2>&1; then
    printf "FAIL [%-4s] %-46s stdout (int) differs\n" "$section" "$label"
    FAIL=$((FAIL+1)); return
  fi

  # 4. Run Corvo (Compiled) if a compiled binary exists
  local tool_name; tool_name=$(echo "$corvo_cmd" | awk '{print $2}' | xargs basename | sed 's/\.corvo//')
  local compiled_bin="/tmp/compiled_${tool_name}/target/debug/corvo_compiled"
  
  if [[ -f "$compiled_bin" ]]; then
    local compiled_ec=0
    local compiled_cmd; compiled_cmd=$(echo "$corvo_cmd" | sed "s|^corvo [^ ]* |$compiled_bin |")
    timeout 10s eval "$compiled_cmd" > /tmp/t_corvo_com.out 2>/tmp/t_corvo_com.err || compiled_ec=$?
    
    if [[ "$gnu_ec" != "$compiled_ec" ]]; then
      printf "FAIL [%-4s] %-46s exit (com): gnu=%s corvo=%s\n" \
        "$section" "$label" "$gnu_ec" "$compiled_ec"
      FAIL=$((FAIL+1)); return
    fi
    if ! diff -q /tmp/t_gnu.out /tmp/t_corvo_com.out >/dev/null 2>&1; then
      printf "FAIL [%-4s] %-46s stdout (com) differs\n" "$section" "$label"
      FAIL=$((FAIL+1)); return
    fi
    printf "PASS [%-4s] %s (int+com)\n" "$section" "$label"
  else
    printf "PASS [%-4s] %s\n" "$section" "$label"
  fi
  
  PASS=$((PASS+1))
}

# Pre-compile a corvo script to a binary in /tmp and verify it links correctly.
prepare_compiled() {
  local script="$1"
  local tool_name; tool_name=$(basename "$script" | sed 's/\.corvo//')
  local out_dir="/tmp/compiled_${tool_name}"
  
  echo "Compiling $script to $out_dir..."
  timeout 60s corvo --transpile "$script" -o "$out_dir" >/dev/null 2>&1 || { echo "ERROR: transpilation failed"; return 1; }
  
  # Ensure Cargo.toml uses the correct path to corvo-lang source inside Docker
  sed -i "s|path = \".*\"|path = \"/corvo/source\"|" "$out_dir/Cargo.toml"
  
  (cd "$out_dir" && timeout 300s cargo build >/dev/null 2>&1) || { echo "ERROR: cargo build failed"; return 1; }
  return 0
}

# Compare uutils vs Corvo: informational only (never fails the suite).
# Args: section label uu_cmd corvo_cmd
run_uutils_case() {
  local section="$1" label="$2" uu_cmd="$3" corvo_cmd="$4"
  local uu_bin; uu_bin="$(echo "$uu_cmd" | awk '{print $1}')"
  command -v "$uu_bin" >/dev/null 2>&1 || return 0
  local uu_ec=0 corvo_ec=0
  timeout 10s eval "$uu_cmd"    > /tmp/u_uu.out    2>/tmp/u_uu.err    || uu_ec=$?
  timeout 10s eval "$corvo_cmd" > /tmp/u_corvo.out 2>/tmp/u_corvo.err || corvo_ec=$?
  if [[ "$uu_ec" != "$corvo_ec" ]] || \
     ! diff -q /tmp/u_uu.out /tmp/u_corvo.out >/dev/null 2>&1; then
    printf "INFO [%-4s] %-46s uutils differs (not required)\n" "$section" "$label"
  else
    printf "INFO [%-4s] %-46s uutils matches\n" "$section" "$label"
  fi
}

# Print wall-clock execution time for a command (informational).
# Args: label cmd [args...]
show_time() {
  local label="$1"; shift
  local start end ms
  start=$(date +%s%N 2>/dev/null) || { echo "INFO: timing unavailable"; return 0; }
  "$@" >/dev/null 2>&1 || true
  end=$(date +%s%N)
  ms=$(( (end - start) / 1000000 ))
  printf "TIME  %-48s %dms\n" "$label" "$ms"
}
