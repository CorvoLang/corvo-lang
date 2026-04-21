#!/usr/bin/env bash
# coreutils/tests/parity/basics.sh
echo "=== basic utilities ==="

run_case arch "plain"             "gnu-arch"                                      "corvo /corvo/coreutils/arch.corvo"
run_case basename "plain"         "gnu-basename /a/b/c.txt"                       "corvo /corvo/coreutils/basename.corvo -- /a/b/c.txt"
run_case basename "suffix"        "gnu-basename /a/b/c.txt .txt"                  "corvo /corvo/coreutils/basename.corvo -- /a/b/c.txt .txt"
run_case dirname "plain"          "gnu-dirname /a/b/c.txt"                        "corvo /corvo/coreutils/dirname.corvo -- /a/b/c.txt"
run_case echo "plain"             "gnu-echo 'hello world'"                         "corvo /corvo/coreutils/echo.corvo -- 'hello world'"
run_case echo "no newline"        "gnu-echo -n 'hello'"                            "corvo /corvo/coreutils/echo.corvo -- -n 'hello'"
run_case pwd "plain"              "cd /fixtures && gnu-pwd"                       "cd /fixtures && corvo /corvo/coreutils/pwd.corvo"
run_case test "TRUE"              "gnu-test 1 -eq 1 && echo yes"                   "corvo /corvo/coreutils/test.corvo -- 1 -eq 1 && echo yes"
run_case test "FALSE"             "gnu-test 1 -eq 2 || echo no"                    "corvo /corvo/coreutils/test.corvo -- 1 -eq 2 || echo no"
run_case expr "add"               "gnu-expr 1 + 2"                                "corvo /corvo/coreutils/expr.corvo -- 1 + 2"
run_case factor "small"           "gnu-factor 12"                                 "corvo /corvo/coreutils/factor.corvo -- 12"
run_case test "-f file"           "gnu-test -f /fixtures/a.txt && echo yes"        "corvo /corvo/coreutils/test.corvo -- -f /fixtures/a.txt && echo yes"
run_case test "-d dir"            "gnu-test -d /fixtures && echo yes"            "corvo /corvo/coreutils/test.corvo -- -d /fixtures && echo yes"
run_case test "-z empty"          "gnu-test -z '' && echo yes"                    "corvo /corvo/coreutils/test.corvo -- -z '' && echo yes"
run_case test "-n nonempty"       "gnu-test -n 'foo' && echo yes"                 "corvo /corvo/coreutils/test.corvo -- -n 'foo' && echo yes"
