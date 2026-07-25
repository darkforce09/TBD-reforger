#!/usr/bin/env bash
# T-181.0 — host-aware execution shim.
#
# WHY THIS EXISTS
# ---------------
# Agent sessions run inside a `debian:12` podman container (`claude-desktop`): glibc 2.36 and
# NO C toolchain at all (no cc/gcc/ld). The real machine is Bazzite / Fedora 44: glibc 2.43
# with gcc. Consequences, both measured:
#
#   * `cargo build` in-container dies with `linker \`cc\` not found`.
#   * Host-built binaries (target/debug/xtask, ArmaReforgerServer, Workbench) refuse to run
#     in-container with `version \`GLIBC_2.39' not found`.
#
# Both failures LOOK like "the repo is broken" and are not. A session that trusts the
# in-container error will "fix" a working toolchain — that happened once already and cost a
# 2.6 GiB `cargo clean`. So: anything that needs a linker, a host glibc, Steam, or a game
# binary goes through `hostrun`.
#
# USAGE
#   source "$(git rev-parse --show-toplevel)/scripts/lib/hostrun.sh"
#   hostrun cargo build -p xtask       # runs on the host when containerised, direct otherwise
#   if in_container; then ...; fi
#
# GOTCHA (measured): under `set -euo pipefail`, `hostrun CMD | head -N` aborts the calling
# script. `head` closes the pipe after N lines, the bridge takes SIGPIPE and reports 127, and
# pipefail turns that into a fatal error even though CMD actually succeeded. Capture first:
#   out="$(hostrun CMD)"; echo "$out" | head -1
# `| tail`, `| cat`, and `| grep` are safe — they drain stdin.
#
# cwd is preserved by distrobox-host-exec, so relative paths behave the same either way.
#
# NOTE: this file is *sourced*, so it deliberately sets no shell options. An earlier revision
# ran `set -uo pipefail` here; those options leaked into the caller and turned an ordinary
# `cmd | head -1` SIGPIPE into a fatal 127. Callers own their own `set -euo pipefail`.

# True when this shell is inside a container (podman/docker/distrobox).
in_container() {
  [ -f /run/.containerenv ] || [ -f /.dockerenv ]
}

# Name of the host bridge, or empty when none is available.
_host_bridge() {
  if command -v distrobox-host-exec >/dev/null 2>&1; then
    echo "distrobox-host-exec"
  elif command -v host-spawn >/dev/null 2>&1; then
    echo "host-spawn"
  fi
}

# Run a command on the host when containerised; run it directly when not.
# Fails loudly with the real diagnosis rather than letting a linker/glibc error mislead.
hostrun() {
  if ! in_container; then
    "$@"
    return $?
  fi

  local bridge
  bridge="$(_host_bridge)"
  if [ -z "$bridge" ]; then
    cat >&2 <<EOF
hostrun: running inside a container with no host bridge available.

  Needed: distrobox-host-exec (or host-spawn) to reach the real machine.
  This container has glibc $(ldd --version 2>/dev/null | head -1 | grep -o '[0-9]\+\.[0-9]\+$' || echo '?') and no C toolchain,
  so '$1' would fail with a misleading linker/GLIBC error rather than a useful one.

  Run this command on the host instead:
      $*
EOF
    return 127
  fi

  "$bridge" "$@"
}

# Assert the host is reachable before a long pipeline starts, so failures land early
# and with the right message.
require_host() {
  if ! in_container; then return 0; fi
  if [ -z "$(_host_bridge)" ]; then
    echo "require_host: no host bridge (distrobox-host-exec/host-spawn) — cannot reach the real machine." >&2
    return 127
  fi
  return 0
}
