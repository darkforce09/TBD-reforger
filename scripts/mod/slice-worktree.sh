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

# The repo uses Git LFS for map assets (packages/map-assets DEM/sat). git-lfs is NOT installed
# in the agent container, so a normal checkout dies with "git-lfs filter-process: not found".
# Slice agents work on mod scripts and never touch those assets, so skip the smudge filter:
# LFS files stay as small pointer files, which also keeps each worktree cheap on disk.
export GIT_LFS_SKIP_SMUDGE=1
# GIT_LFS_SKIP_SMUDGE alone is not enough: git still tries to SPAWN the filter process, which
# does not exist. Neutralise the filter config for our own git calls instead.
#
# `core.hooksPath=/dev/null` is load-bearing, not tidiness. The repo's post-checkout / post-merge
# hooks are LFS hooks, and without git-lfs installed they exit non-zero — which makes
# `git worktree add` return 2 EVEN THOUGH THE WORKTREE WAS CREATED SUCCESSFULLY. Combined with
# `set -e` at the top of this script, that killed `new` immediately after the tree appeared and
# before the oracle symlinks were made, silently, with a real-looking "Preparing worktree" line as
# the last output. Every worktree this factory produced was missing crf_framework and
# vanilla_reference, so slice agents lost both proof lanes and had nothing to check Enfusion
# claims against. Measured: exit 2 with hooks, exit 0 without.
GIT=(git -c core.hooksPath=/dev/null -c filter.lfs.smudge= -c filter.lfs.process= -c filter.lfs.clean=cat -c filter.lfs.required=false)

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
    # An existing tree is NOT skipped outright — it falls through to the oracle link step, which
    # is idempotent. Early-returning here is what made the missing-oracle bug unrepairable: the
    # trees existed, so every subsequent `new`/`prep` said "already exists" and changed nothing.
    if [ -d "$dir" ]; then
      echo "already exists: $dir (re-checking oracles)"
    else
      mkdir -p "$BASE"
      # Branch from the CURRENT main tip so the agent gets the committed factory.
      if git show-ref --verify --quiet "refs/heads/$branch"; then
        "${GIT[@]}" worktree add "$dir" "$branch"
      else
        "${GIT[@]}" worktree add -b "$branch" "$dir" main
      fi
    fi
    # The oracle sources are GITIGNORED, so a fresh worktree has neither. Without them an
    # agent cannot query CRF or read vanilla source and will fall back on training-data guesses
    # about Enfusion — which are wrong. Link them from the main tree (read-only reference; no
    # disk cost, no risk of a slice mutating them).
    missing_oracle=0
    for ref in crf_framework vanilla_reference; do
      if [ ! -d "$ROOT/apps/mod/$ref" ]; then
        echo "  WARNING: $ROOT/apps/mod/$ref missing on main — cannot link that oracle lane" >&2
        missing_oracle=1
        continue
      fi
      ln -sfn "$ROOT/apps/mod/$ref" "$dir/apps/mod/$ref"
      # Verify rather than trust. This whole block used to be unreachable and nothing noticed,
      # because nobody checked the result — the agents just quietly lost their proof lanes.
      if [ -d "$dir/apps/mod/$ref" ]; then
        echo "  oracle ok: apps/mod/$ref"
      else
        echo "  ERROR: failed to link apps/mod/$ref into $dir" >&2
        missing_oracle=1
      fi
    done
    if [ "$missing_oracle" -ne 0 ]; then
      echo "REFUSING: $dir has no usable oracle lane — an agent here would guess at Enfusion." >&2
      exit 1
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
