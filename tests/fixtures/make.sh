#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Builds deterministic fixture repositories under tests/fixtures/out/.
# Usage: tests/fixtures/make.sh [name...]   (default: all small fixtures)
#        tests/fixtures/make.sh big-10k big-100k
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT="$ROOT/out"
mkdir -p "$OUT"

export GIT_AUTHOR_NAME="Ada Fixture" GIT_AUTHOR_EMAIL="ada@fixture.test"
export GIT_COMMITTER_NAME="Ada Fixture" GIT_COMMITTER_EMAIL="ada@fixture.test"
export GIT_CONFIG_NOSYSTEM=1 HOME="$OUT/.home"
mkdir -p "$HOME"
TICK=1700000000

new_repo() {
  TICK=1700000000
  rm -rf "$OUT/$1"
  git init -q -b main "$OUT/$1"
  cd "$OUT/$1"
  git config commit.gpgsign false
  git config user.name "$GIT_AUTHOR_NAME"
  git config user.email "$GIT_AUTHOR_EMAIL"
}

# commit <file> <line> <message>: append a line and commit with the next fixed timestamp.
commit() {
  TICK=$((TICK + 3600))
  mkdir -p "$(dirname "$1")"
  echo "$2" >> "$1"
  git add -A
  GIT_AUTHOR_DATE="@$TICK +0000" GIT_COMMITTER_DATE="@$TICK +0000" git commit -q -m "$3"
}

merge() {
  TICK=$((TICK + 3600))
  GIT_AUTHOR_DATE="@$TICK +0000" GIT_COMMITTER_DATE="@$TICK +0000" \
    git merge -q --no-ff -m "Merge $*" "$@"
}

fx_linear() {
  new_repo linear
  for i in 1 2 3 4 5; do commit file.txt "line $i" "Commit $i"; done
}

fx_merges() {
  new_repo merges
  commit a.txt a "Initial"
  git checkout -q -b feature/a; commit a.txt a1 "A1"; commit a.txt a2 "A2"
  git checkout -q main; commit m.txt m1 "M1"
  git checkout -q -b feature/b; commit b.txt b1 "B1"
  git checkout -q main; merge feature/a
  commit m.txt m2 "M2"
  merge feature/b
}

fx_octopus() {
  new_repo octopus
  commit base.txt base "Base"
  for b in x y z; do git checkout -q -b "br-$b" main; commit "$b.txt" "$b" "On $b"; done
  git checkout -q main
  merge br-x br-y br-z
}

fx_criss_cross() {
  new_repo criss-cross
  commit f.txt 0 "Root"
  git checkout -q -b side
  commit s.txt 1 "Side 1"
  git checkout -q main; commit m.txt 1 "Main 1"
  merge side
  git checkout -q side; merge main~1
  commit s.txt 2 "Side 2"
  git checkout -q main; commit m.txt 2 "Main 2"
  merge side
}

fx_stash_wip() {
  new_repo stash-wip
  commit app.txt v1 "App v1"
  commit app.txt v2 "App v2"
  echo "stashed" >> app.txt
  TICK=$((TICK + 3600))
  GIT_AUTHOR_DATE="@$TICK +0000" GIT_COMMITTER_DATE="@$TICK +0000" git stash push -q -m "WIP stash"
  echo "unstaged" >> app.txt
  echo "new" > new.txt
}

# Same shape as the demo repo used in the GitKraken research.
fx_demo() {
  new_repo demo
  commit README.md "# Demo" "Initial commit"
  commit src/app.txt "app v1" "Add app skeleton"
  git checkout -q -b feature/login; commit src/login.txt "login form" "Add login form"
  git checkout -q main; commit src/app.txt "app v2" "Improve app startup"
  git checkout -q feature/login; commit src/login.txt "validation" "Validate login input"
  git checkout -q -b feature/graph main; commit src/graph.txt "lanes" "Draft lane layout"
  git checkout -q main; merge feature/login
  TICK=$((TICK + 60)); GIT_COMMITTER_DATE="@$TICK +0000" git tag -a v0.1.0 -m "First release"
  git checkout -q feature/graph; commit src/graph.txt "edges" "Draw graph edges"
  git checkout -q main; commit docs/guide.md "guide" "Write user guide"
  git checkout -q -b fix/typo main~1; commit README.md "typo fix" "Fix typo in README"
  git checkout -q feature/graph; commit src/graph.txt "virtual rows" "Virtualize graph rows"
  git checkout -q main; merge feature/graph
  commit src/app.txt "app v3" "Bump version"
  git checkout -q -b feature/conflict main~1
  echo "conflicting line" > src/app.txt; git add -A
  TICK=$((TICK + 3600)); GIT_AUTHOR_DATE="@$TICK +0000" GIT_COMMITTER_DATE="@$TICK +0000" git commit -q -m "Change app text differently"
  git checkout -q main
}

# big-N: N commits via fast-import. Shape: a main line with a feature branch
# merged every 50 commits (branch of 5 commits), deterministic.
fx_big() {
  local n="$1" name="big-$2"
  new_repo "$name"
  python3 - "$n" <<'PY' | git fast-import --quiet
import sys
n = int(sys.argv[1])
t = 1600000000
mark = 0
out = sys.stdout
def commit(ref, msg, parents, content):
    global mark, t
    mark += 1; t += 60
    data = msg.encode()
    out.write(f"commit {ref}\nmark :{mark}\n")
    out.write(f"author Ada Fixture <ada@fixture.test> {t} +0000\n")
    out.write(f"committer Ada Fixture <ada@fixture.test> {t} +0000\n")
    out.write(f"data {len(data)}\n{msg}\n")
    if parents:
        out.write(f"from :{parents[0]}\n")
        for p in parents[1:]:
            out.write(f"merge :{p}\n")
    out.write(f"M 644 inline f{mark % 97}.txt\ndata {len(content)}\n{content}\n\n")
    return mark
head = commit("refs/heads/main", "Commit 1", [], "1")
count = 1
while count < n:
    if count % 50 == 0 and count + 6 <= n:
        b = head
        for k in range(5):
            b = commit("refs/heads/feature", f"Feature {count}-{k}", [b], f"feat {count} {k}")
            count += 1
        head = commit("refs/heads/main", f"Merge feature {count}", [head, b], f"merge {count}")
        count += 1
    else:
        head = commit("refs/heads/main", f"Commit {count + 1}", [head], str(count + 1))
        count += 1
PY
  git checkout -q -f main
}

targets=("$@")
[ ${#targets[@]} -eq 0 ] && targets=(linear merges octopus criss-cross stash-wip demo)
for t in "${targets[@]}"; do
  case "$t" in
    linear) fx_linear ;; merges) fx_merges ;; octopus) fx_octopus ;;
    criss-cross) fx_criss_cross ;; stash-wip) fx_stash_wip ;; demo) fx_demo ;;
    big-10k) fx_big 10000 10k ;; big-100k) fx_big 100000 100k ;;
    *) echo "unknown fixture: $t" >&2; exit 2 ;;
  esac
  echo "built $t: $(git -C "$OUT/$t" rev-list --all --count) commits"
done
