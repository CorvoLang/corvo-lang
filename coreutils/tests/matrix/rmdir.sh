#!/usr/bin/env bash
# coreutils/tests/matrix/rmdir.sh
# Extended flag-combination matrix for rmdir.
# Sourced by matrix/run-all.sh; requires run_case and PASS / FAIL in scope.

echo "=== rmdir ==="

_MTD="$(mktemp -d)"
# shellcheck disable=SC2064
trap "rm -rf '$_MTD'" EXIT

# ── Error cases ───────────────────────────────────────────────────────────────
run_case rmdir "no operand" \
  "gnu-rmdir" \
  "corvo /corvo/coreutils/rmdir.corvo"

run_case rmdir "nonexistent directory" \
  "gnu-rmdir '$_MTD/no_such'" \
  "corvo /corvo/coreutils/rmdir.corvo -- '$_MTD/no_such'"

mkdir -p "$_MTD/nonempty_gnu" "$_MTD/nonempty_corvo"
echo "x" > "$_MTD/nonempty_gnu/f.txt"
echo "x" > "$_MTD/nonempty_corvo/f.txt"
run_case rmdir "non-empty directory (error)" \
  "gnu-rmdir '$_MTD/nonempty_gnu'" \
  "corvo /corvo/coreutils/rmdir.corvo -- '$_MTD/nonempty_corvo'"

# ── Successful removal ────────────────────────────────────────────────────────
mkdir -p "$_MTD/empty_gnu" "$_MTD/empty_corvo"
_gnu_ec=0; _corvo_ec=0
gnu-rmdir "$_MTD/empty_gnu"   >/dev/null 2>&1 || _gnu_ec=$?
corvo /corvo/coreutils/rmdir.corvo -- "$_MTD/empty_corvo" >/dev/null 2>&1 || _corvo_ec=$?
if [[ "$_gnu_ec" == "$_corvo_ec" ]] && \
   [[ ! -d "$_MTD/empty_gnu" ]] && [[ ! -d "$_MTD/empty_corvo" ]]; then
  printf "PASS [rmdir] remove empty directory\n"; PASS=$((PASS+1))
else
  printf "FAIL [rmdir] remove empty directory  exit: gnu=%s corvo=%s\n" \
    "$_gnu_ec" "$_corvo_ec"
  FAIL=$((FAIL+1))
fi

# -p: nested parents removed
mkdir -p "$_MTD/gnu_p/b/c" "$_MTD/corvo_p/b/c"
_gnu_p_ec=0; _corvo_p_ec=0
gnu-rmdir -p "$_MTD/gnu_p/b/c"   >/dev/null 2>&1 || _gnu_p_ec=$?
corvo /corvo/coreutils/rmdir.corvo -- -p "$_MTD/corvo_p/b/c" >/dev/null 2>&1 || _corvo_p_ec=$?
if [[ "$_gnu_p_ec" == "$_corvo_p_ec" ]] && \
   [[ ! -d "$_MTD/gnu_p" ]] && [[ ! -d "$_MTD/corvo_p" ]]; then
  printf "PASS [rmdir] parents (-p) removes ancestors\n"; PASS=$((PASS+1))
else
  printf "FAIL [rmdir] parents (-p) removes ancestors  exit: gnu=%s corvo=%s\n" \
    "$_gnu_p_ec" "$_corvo_p_ec"
  FAIL=$((FAIL+1))
fi

# -p stops at non-empty parent
mkdir -p "$_MTD/stop_gnu/b/c" "$_MTD/stop_corvo/b/c"
echo "sibling" > "$_MTD/stop_gnu/sib.txt"
echo "sibling" > "$_MTD/stop_corvo/sib.txt"
_gnu_stop_ec=0; _corvo_stop_ec=0
gnu-rmdir -p "$_MTD/stop_gnu/b/c"   >/dev/null 2>&1 || _gnu_stop_ec=$?
corvo /corvo/coreutils/rmdir.corvo -- -p "$_MTD/stop_corvo/b/c" >/dev/null 2>&1 || _corvo_stop_ec=$?
if [[ "$_gnu_stop_ec" == "$_corvo_stop_ec" ]] && \
   [[ -d "$_MTD/stop_gnu" ]] && [[ -d "$_MTD/stop_corvo" ]]; then
  printf "PASS [rmdir] parents (-p) stops at non-empty\n"; PASS=$((PASS+1))
else
  printf "FAIL [rmdir] parents (-p) stops at non-empty  exit: gnu=%s corvo=%s\n" \
    "$_gnu_stop_ec" "$_corvo_stop_ec"
  FAIL=$((FAIL+1))
fi

# --ignore-fail-on-non-empty: exit 0 even for non-empty
mkdir -p "$_MTD/igne_gnu" "$_MTD/igne_corvo"
echo "x" > "$_MTD/igne_gnu/f.txt"
echo "x" > "$_MTD/igne_corvo/f.txt"
_gnu_ig_ec=0; _corvo_ig_ec=0
gnu-rmdir --ignore-fail-on-non-empty "$_MTD/igne_gnu"   >/dev/null 2>&1 || _gnu_ig_ec=$?
corvo /corvo/coreutils/rmdir.corvo -- --ignore-fail-on-non-empty "$_MTD/igne_corvo" >/dev/null 2>&1 || _corvo_ig_ec=$?
if [[ "$_gnu_ig_ec" == "$_corvo_ig_ec" ]]; then
  printf "PASS [rmdir] --ignore-fail-on-non-empty exit code\n"; PASS=$((PASS+1))
else
  printf "FAIL [rmdir] --ignore-fail-on-non-empty exit code  gnu=%s corvo=%s\n" \
    "$_gnu_ig_ec" "$_corvo_ig_ec"
  FAIL=$((FAIL+1))
fi

# Verbose (-v): same path so output matches
mkdir -p "$_MTD/vtest_rmdir"
gnu-rmdir -v "$_MTD/vtest_rmdir" > /tmp/rmdir_m_gnu_v.out 2>/dev/null || true
sed -i 's/^gnu-rmdir/rmdir/' /tmp/rmdir_m_gnu_v.out
mkdir -p "$_MTD/vtest_rmdir"
corvo /corvo/coreutils/rmdir.corvo -- -v "$_MTD/vtest_rmdir" > /tmp/rmdir_m_corvo_v.out 2>/dev/null || true
if diff -q /tmp/rmdir_m_gnu_v.out /tmp/rmdir_m_corvo_v.out >/dev/null 2>&1; then
  printf "PASS [rmdir] verbose (-v) output\n"; PASS=$((PASS+1))
else
  printf "FAIL [rmdir] verbose (-v) output\n"
  diff -u /tmp/rmdir_m_gnu_v.out /tmp/rmdir_m_corvo_v.out | head -10 || true
  FAIL=$((FAIL+1))
fi
