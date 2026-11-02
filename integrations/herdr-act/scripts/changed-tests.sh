#!/bin/sh
# Picks the test files a change since a commit reaches, for the shorter check of a fix round (`npm run check -- --since
# <commit>`, scripts/check.sh): one module of tests/main.rs a line, or the one word `all`, with why on stderr, where it
# cannot tell. `sh scripts/changed-tests.sh <commit>`. What reaches what: docs/testing.md.
set -e
all() { echo "Every test runs: $1" >&2; echo all; exit 0; }
rev="$1"
[ -n "$rev" ] || all "no commit to start from."
git rev-parse -q --verify "$rev^{commit}" >/dev/null 2>&1 || all "$rev is no commit here."
git merge-base --is-ancestor "$rev" HEAD 2>/dev/null || all "the branch no longer holds $rev."
# What changed: committed since, not committed yet, and new files.
changed=$( { git diff --name-only "$rev" -- && git ls-files --others --exclude-standard; } | sort -u)
out=$(mktemp)
trap 'rm -f "$out"' EXIT
count() { echo $(( $(wc -l < "$out") )); }
pick() { [ -f "tests/$1.rs" ] && [ "$1" != main ] && echo "$1" >> "$out" || true; }
# The test files whose names hold this word (`_` between words), and those that write this text.
by_word() { for t in tests/*.rs; do m=$(basename "$t" .rs); case "_${m}_" in *"_$1_"*) pick "$m" ;; esac; done; }
by_text() { grep -lF -- "$1" tests/*.rs 2>/dev/null | while read -r t; do pick "$(basename "$t" .rs)"; done; }
for f in $changed; do
  before=$(count)
  case "$f" in
    tests/main.rs|tests/common/*|tests/fixtures/*) all "$f is shared by every test." ;;
    tests/frames/*) pick board_frames ;;
    tests/*/*) d=${f#tests/}; d=${d%%/*}; by_text "crate::$d" ;;
    tests/*.rs) pick "$(basename "$f" .rs)" ;;
    src/lib.rs|src/main.rs) all "$f holds the whole program." ;;
    crates/*/src/lib.rs) all "$f is the root of a crate (docs/crates.md): it names all of that crate's code." ;;
    src/*|crates/*/src/*)
      # A file of the program: the test files named by it (its name, its folder's) or that write its module's path.
      stem=$(basename "$f"); stem=${stem%.*}
      dir=$(dirname "$f"); parent=$(basename "$dir")
      [ "$stem" = mod ] && { stem=$parent; dir=$(dirname "$dir"); parent=$(basename "$dir"); }
      [ "$parent" = src ] && parent=sw # the top of a crate's code (src/, crates/<crate>/src/)
      by_word "$stem"
      [ "$parent" = sw ] || by_word "$parent"
      by_text "$parent::$stem"
      [ "$(count)" -gt "$before" ] || all "no test file is named by $f or writes its path."
      ;;
    docs/*|README.md|AGENTS.md) by_text "$(basename "$f")"; pick guard ;;
    *) all "$f is not a test, the program or a page: what it reaches is not known." ;;
  esac
done
total=$(ls tests/*.rs | grep -vc '^tests/main.rs$')
picked=$(sort -u "$out")
[ "$(printf '%s\n' "$picked" | grep -c .)" -lt "$total" ] || all "the change reaches every test file."
printf '%s\n' "$picked" | grep . || true
