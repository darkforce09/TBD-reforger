#!/usr/bin/env bash
# Slice worktree lifecycle — see docs/mod/SLICE_WORKFLOW.md (operator-defined, binding).
#
# One worktree per SLICE. Sub-slices (T-181.7.1) live in their parent's worktree (T-181.7),
# because they are the same slice's work. Three worktrees at a time; merge when all three are
# complete; DELETE immediately after merging — leftover trees fill the disk.
#
#   bash scripts/mod/slice-worktree.sh new   T-181.7
#   bash scripts/mod/slice-worktree.sh list
#   bash scripts/mod/slice-worktree.sh merge T-181.7
#   bash scripts/mod/slice-worktree.sh drop  T-181.7
#   bash scripts/mod/slice-worktree.sh reap
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
BASE=".ai/artifacts/worktrees"

# A sub-slice belongs to its parent's tree: T-181.7.1 -> T-181.7. T-181.7 stays T-181.7.
parent_slice() {
  local s="$1"
  # Keep at most PROGRAM.SLICE (two dot-separated parts after the T-xxx).
  echo "$s" | sed -E 's/^(T-[0-9]+\.[0-9]+).*/\1/'
}

usage() { sed -n '2,14p' "$0"; }

cmd="${1:-}"; slice="${2:-}"

case "$cmd" in
  new)
    [ -n "$slice" ] || { echo "usage: $0 new <slice>" >&2; exit 2; }
    p="$(parent_slice "$slice")"
    if [ "$p" != "$slice" ]; then
      echo "note: $slice is a sub-slice — it belongs in $p's worktree (SLICE_WORKFLOW.md rule 1)"
      slice="$p"
    fi
    dir="$BASE/$slice"
    branch="slice/$slice"
    if [ -d "$dir" ]; then echo "already exists: $dir"; exit 0; fi
    mkdir -p "$BASE"
    # Branch from the CURRENT main tip so the agent gets the committed factory.
    if git show-ref --verify --quiet "refs/heads/$branch"; then
      git worktree add "$dir" "$branch"
    else
      git worktree add -b "$branch" "$dir" main
    fi
    echo "worktree: $ROOT/$dir   branch: $branch"
    ;;

  list)
    git worktree list
    ;;

  merge)
    [ -n "$slice" ] || { echo "usage: $0 merge <slice>" >&2; exit 2; }
    slice="$(parent_slice "$slice")"
    branch="slice/$slice"
    dir="$BASE/$slice"
    [ -d "$dir" ] || { echo "no worktree at $dir" >&2; exit 1; }
    # Refuse to merge a dirty tree — uncommitted slice work would be silently lost.
    if [ -n "$(git -C "$dir" status --porcelain)" ]; then
      echo "REFUSING: $dir has uncommitted changes. Commit them in the worktree first:" >&2
      git -C "$dir" status --short >&2
      exit 1
    fi
    git merge --no-ff "$branch" -m "T-181: merge $branch

Co-Authored-By: Claude <noreply@anthropic.com>"
    echo "merged $branch -> main"
    ;;

  drop)
    [ -n "$slice" ] || { echo "usage: $0 drop <slice>" >&2; exit 2; }
    slice="$(parent_slice "$slice")"
    dir="$BASE/$slice"; branch="slice/$slice"
    git worktree remove --force "$dir" 2>/dev/null || rm -rf "$dir"
    git branch -D "$branch" 2>/dev/null || true
    git worktree prune
    echo "dropped $dir + $branch"
    ;;

  reap)
    # Delete every slice worktree whose branch is already merged into main — the
    # post-wave cleanup step. Disk is the constraint; do not skip this.
    n=0
    for d in "$BASE"/*/; do
      [ -d "$d" ] || continue
      s="$(basename "$d")"; b="slice/$s"
      if git show-ref --verify --quiet "refs/heads/$b" && git merge-base --is-ancestor "$b" main 2>/dev/null; then
        git worktree remove --force "$d" 2>/dev/null || rm -rf "$d"
        git branch -d "$b" 2>/dev/null || true
        echo "reaped $s (merged)"
        n=$((n+1))
      else
        echo "kept   $s (not merged into main)"
      fi
    done
    git worktree prune
    echo "reaped $n worktree(s)"
    df -h "$ROOT" | tail -1
    ;;

  *) usage; exit 2 ;;
esac
