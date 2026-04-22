#!/usr/bin/env bash
# coreutils/tests/matrix/mkdir.sh
# Extended flag-combination matrix for mkdir.
# Sourced by matrix/run-all.sh; requires run_case and PASS / FAIL in scope.

echo "=== mkdir ==="

_MTD="$(mktemp -d)"
# shellcheck disable=SC2064
trap "rm -rf '$_MTD'" EXIT

# ── Error cases ───────────────────────────────────────────────────────────────
run_case mkdir "no operand" \
  "gnu-mkdir" \
  "corvo /corvo/coreutils/mkdir.corvo"

mkdir -p "$_MTD/existing_gnu" "$_MTD/existing_corvo"
run_case mkdir "existing directory (error)" \
  "gnu-mkdir '$_MTD/existing_gnu'" \
  "corvo /corvo/coreutils/mkdir.corvo -- '$_MTD/existing_corvo'"

run_case mkdir "missing parent (error)" \
  "gnu-mkdir '$_MTD/no_parent/subdir'" \
  "corvo /corvo/coreutils/mkdir.corvo -- '$_MTD/no_parent/subdir'"

# ── Successful creation (filesystem state checks) ────────────────────────────
_gnu_ec=0; _corvo_ec=0
gnu-mkdir "$_MTD/gnu_new"   >/dev/null 2>&1 || _gnu_ec=$?
corvo /corvo/coreutils/mkdir.corvo -- "$_MTD/corvo_new" >/dev/null 2>&1 || _corvo_ec=$?
if [[ "$_gnu_ec" == "$_corvo_ec" ]] && \
   [[ -d "$_MTD/gnu_new" ]] && [[ -d "$_MTD/corvo_new" ]]; then
  printf "PASS [mkdir] create new directory\n"; PASS=$((PASS+1))
else
  printf "FAIL [mkdir] create new directory  exit: gnu=%s corvo=%s\n" \
    "$_gnu_ec" "$_corvo_ec"
  FAIL=$((FAIL+1))
fi

# -p: no error when directory already exists
mkdir -p "$_MTD/exist_p_gnu" "$_MTD/exist_p_corvo"
run_case mkdir "existing dir with -p (no error)" \
  "gnu-mkdir -p '$_MTD/exist_p_gnu'" \
  "corvo /corvo/coreutils/mkdir.corvo -- -p '$_MTD/exist_p_corvo'"

# -p: create nested directories
_gnu_p_ec=0; _corvo_p_ec=0
gnu-mkdir -p "$_MTD/gnu_nested/b/c"   >/dev/null 2>&1 || _gnu_p_ec=$?
corvo /corvo/coreutils/mkdir.corvo -- -p "$_MTD/corvo_nested/b/c" >/dev/null 2>&1 || _corvo_p_ec=$?
if [[ "$_gnu_p_ec" == "$_corvo_p_ec" ]] && \
   [[ -d "$_MTD/gnu_nested/b/c" ]] && [[ -d "$_MTD/corvo_nested/b/c" ]]; then
  printf "PASS [mkdir] create nested dirs (-p)\n"; PASS=$((PASS+1))
else
  printf "FAIL [mkdir] create nested dirs (-p)  exit: gnu=%s corvo=%s\n" \
    "$_gnu_p_ec" "$_corvo_p_ec"
  FAIL=$((FAIL+1))
fi

# Multiple directories at once
_gnu_m_ec=0; _corvo_m_ec=0
gnu-mkdir "$_MTD/gnu_a" "$_MTD/gnu_b"   >/dev/null 2>&1 || _gnu_m_ec=$?
corvo /corvo/coreutils/mkdir.corvo -- "$_MTD/corvo_a" "$_MTD/corvo_b" >/dev/null 2>&1 || _corvo_m_ec=$?
if [[ "$_gnu_m_ec" == "$_corvo_m_ec" ]] && \
   [[ -d "$_MTD/gnu_a" ]] && [[ -d "$_MTD/gnu_b" ]] && \
   [[ -d "$_MTD/corvo_a" ]] && [[ -d "$_MTD/corvo_b" ]]; then
  printf "PASS [mkdir] multiple directories\n"; PASS=$((PASS+1))
else
  printf "FAIL [mkdir] multiple directories  exit: gnu=%s corvo=%s\n" \
    "$_gnu_m_ec" "$_corvo_m_ec"
  FAIL=$((FAIL+1))
fi

# Verbose (-v): same path so output matches
gnu-mkdir -v "$_MTD/vtest_mkdir" > /tmp/mkdir_m_gnu_v.out 2>/dev/null || true
sed -i 's/^gnu-mkdir/mkdir/' /tmp/mkdir_m_gnu_v.out
rm -rf "$_MTD/vtest_mkdir"
corvo /corvo/coreutils/mkdir.corvo -- -v "$_MTD/vtest_mkdir" > /tmp/mkdir_m_corvo_v.out 2>/dev/null || true
if diff -q /tmp/mkdir_m_gnu_v.out /tmp/mkdir_m_corvo_v.out >/dev/null 2>&1; then
  printf "PASS [mkdir] verbose (-v) output\n"; PASS=$((PASS+1))
else
  printf "FAIL [mkdir] verbose (-v) output\n"
  diff -u /tmp/mkdir_m_gnu_v.out /tmp/mkdir_m_corvo_v.out | head -10 || true
  FAIL=$((FAIL+1))
fi

# -m (mode): accepted silently, directory still created
_gnu_m_ec=0; _corvo_m_ec=0
gnu-mkdir -m 755 "$_MTD/gnu_mode"   >/dev/null 2>&1 || _gnu_m_ec=$?
corvo /corvo/coreutils/mkdir.corvo -- -m 755 "$_MTD/corvo_mode" >/dev/null 2>&1 || _corvo_m_ec=$?
if [[ "$_gnu_m_ec" == "$_corvo_m_ec" ]] && \
   [[ -d "$_MTD/gnu_mode" ]] && [[ -d "$_MTD/corvo_mode" ]]; then
  printf "PASS [mkdir] mode flag (-m) accepted\n"; PASS=$((PASS+1))
else
  printf "FAIL [mkdir] mode flag (-m)  exit: gnu=%s corvo=%s\n" \
    "$_gnu_m_ec" "$_corvo_m_ec"
  FAIL=$((FAIL+1))
fi
