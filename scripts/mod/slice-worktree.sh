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

    # NOTE for anyone tempted to "fix" the LFS pointers here: DON'T symlink them.
    # LFS content is deliberately not smudged in a worktree (see the filter neutralisation at the
    # top of this file), so `packages/map-assets/**` arrives as ~133-byte pointer files. That makes
    # `make schema-validate` die inside a worktree at its `schema height-labels` step ("PNG decode:
    # Invalid PNG signature") while passing on main, and two separate agents burned real effort
    # diagnosing it. Symlinking the real assets in DOES fix the target — and was tried and reverted,
    # because git then reports all 983 tracked files under that path as DELETED, which leaves every
    # worktree permanently dirty and `wave.sh land` refuses to merge a dirty tree. Hiding that with
    # `git update-index --skip-worktree` would make working-tree changes INVISIBLE, which in a
    # program that merges unattended agent work is a way to silently lose a slice.
    # The pointers are correct and cheap. `make schema-validate` is a MAIN-TREE target; inside a
    # worktree run the specific sub-gate instead: `cargo run -p xtask -- schema validate`.
    echo "  note: packages/map-assets is LFS pointers here — run 'xtask schema validate', not 'make schema-validate'"
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
    # ── THIS DESTROYED FIVE LIVE AGENTS' WORK ONCE. Read before changing. ────────────────────
    # The old test was `merge-base --is-ancestor <branch> main` alone, which is TRUE for a branch
    # that has NO COMMITS AT ALL — a freshly-created worktree sits exactly at the branch point,
    # and the branch point is trivially an ancestor of main. So "no work done yet" was
    # indistinguishable from "work merged". Combined with `worktree remove --force`, which
    # deliberately overrides git's own refusal to delete a dirty tree, a single reap wiped five
    # worktrees whose agents were mid-slice with uncommitted files. Nothing in git could recover
    # them; only the agents' own context could.
    # Three independent guards now, because any one of them alone would have prevented it:
    #   1. never reap a tree with uncommitted changes or untracked files
    #   2. never reap a branch that has no commits beyond main (unstarted != merged)
    #   3. no --force, so git's own safety check is a backstop rather than something we suppress
    n=0
    for d in "$BASE"/*/; do
      [ -d "$d" ] || continue
      s="$(basename "$d")"; b="slice/$s"

      if ! git show-ref --verify --quiet "refs/heads/$b"; then
        echo "kept   $s (no branch)"
        continue
      fi

      dirty="$("${GIT[@]}" -C "$d" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
      if [ "${dirty:-0}" != "0" ]; then
        echo "KEPT   $s — $dirty uncommitted change(s). An agent may still be working here." >&2
        continue
      fi

      # "No commits beyond main" is ambiguous: it means UNSTARTED (fresh tree, nothing written)
      # AND it means MERGED (the commits are now in main). Those need opposite outcomes, and
      # conflating them is what destroyed five worktrees. The distinguisher is main's own history:
      # `merge` leaves a "Merge branch 'slice/<id>'" commit, so if that exists the work landed.
      commits="$(git rev-list --count "main..$b" 2>/dev/null || echo 0)"
      landed="$(git log main --merges --oneline --grep="slice/$s\$" 2>/dev/null | head -1)"
      if [ "${commits:-0}" = "0" ] && [ -z "$landed" ]; then
        echo "KEPT   $s — no commits beyond main and no merge in main's history (unstarted)." >&2
        continue
      fi

      if ! git merge-base --is-ancestor "$b" main 2>/dev/null; then
        echo "kept   $s ($commits commit(s) not merged into main)"
        continue
      fi

      if git worktree remove "$d" 2>/dev/null; then
        git branch -d "$b" 2>/dev/null || true
        echo "reaped $s (merged, clean)"
        n=$((n+1))
      else
        echo "KEPT   $s — git refused to remove it; inspect by hand." >&2
      fi
    done
    git worktree prune
    echo "reaped $n worktree(s)"
    df -h "$ROOT" | tail -1
    ;;

  *) usage; exit 2 ;;
esac
