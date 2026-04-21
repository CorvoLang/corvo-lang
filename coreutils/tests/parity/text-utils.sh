#!/usr/bin/env bash
# coreutils/tests/parity/text-utils.sh
echo "=== text utilities ==="

printf "line1\nline2\n" > /tmp/text_a.txt
printf "col1\ncol2\n" > /tmp/text_b.txt

run_case nl "plain"              "gnu-nl /fixtures/a.txt"                       "corvo /corvo/coreutils/nl.corvo -- /fixtures/a.txt"
run_case tac "plain"              "gnu-tac /fixtures/a.txt"                      "corvo /corvo/coreutils/tac.corvo -- /fixtures/a.txt"
run_case tee "plain"              "printf 'hi\n' | gnu-tee /tmp/tee.out"         "printf 'hi\n' | corvo /corvo/coreutils/tee.corvo -- /tmp/tee_corvo.out"
# Check tee output file parity
run_case tee-file "output match" "cat /tmp/tee.out" "cat /tmp/tee_corvo.out"

run_case paste "merge"            "gnu-paste /tmp/text_a.txt /tmp/text_b.txt"    "corvo /corvo/coreutils/paste.corvo -- /tmp/text_a.txt /tmp/text_b.txt"
run_case sum "plain"              "gnu-sum /fixtures/a.txt"                      "corvo /corvo/coreutils/sum.corvo -- /fixtures/a.txt"

printf "a b\nb c\n" > /tmp/tsort_in.txt
run_case tsort "basic"            "gnu-tsort /tmp/tsort_in.txt"                  "corvo /corvo/coreutils/tsort.corvo -- /tmp/tsort_in.txt"

run_case fmt "basic"              "gnu-fmt /fixtures/long.txt"                   "corvo /corvo/coreutils/fmt.corvo -- /fixtures/long.txt"
run_case fold "basic"             "gnu-fold -w 5 /fixtures/a.txt"                "corvo /corvo/coreutils/fold.corvo -- -w 5 /fixtures/a.txt"

rm -f /tmp/text_a.txt /tmp/text_b.txt /tmp/tee.out /tmp/tee_corvo.out /tmp/tsort_in.txt
