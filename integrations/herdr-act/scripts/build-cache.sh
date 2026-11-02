#!/bin/sh
# The build cache every worktree of this repo shares on a machine: sccache, one pinned version checked against its
# published sha256, kept with its cache in ~/.cache/herdr-projects (outside every worktree, so removing one frees
# nothing shared). `sh scripts/build-cache.sh` installs it (the workspace setup does); `. scripts/build-cache.sh` in a
# script that runs cargo (scripts/check.sh, scripts/setup.sh) compiles through it when it is installed, and changes
# nothing when it is not (CI, offline, another platform). How and why: docs/testing.md.
bc_version=0.18.0
bc_dir="${XDG_CACHE_HOME:-$HOME/.cache}/herdr-projects"
bc_bin="$bc_dir/sccache-$bc_version/sccache"
bc_install() {
  [ -x "$bc_bin" ] && return 0
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) t=x86_64-unknown-linux-musl; sum=45f1447fbe231e3037bde351ef70677dd212216c8d62ae7ca409fecc4d6acc89 ;;
    Linux-aarch64|Linux-arm64) t=aarch64-unknown-linux-musl; sum=2b3284d5da3b46a47dc4229e75bb7b88ac4aa99c8d754fb7d2f84997e5a4354a ;;
    Darwin-x86_64) t=x86_64-apple-darwin; sum=1dade83cc49eeb42337565eccd534b05982820a8e44b851bd7937467a18c7aef ;;
    Darwin-arm64) t=aarch64-apple-darwin; sum=308184519b646f5125289e8515b36f6ca65a13a041923994aebe702348674e8e ;;
    *) echo "No shared build cache for $(uname -s) $(uname -m): this worktree builds on its own." >&2; return 0 ;;
  esac
  name="sccache-v$bc_version-$t"; url="https://github.com/mozilla/sccache/releases/download/v$bc_version/$name.tar.gz"
  tmp=$(mktemp -d) || return 0
  if command -v curl >/dev/null 2>&1; then curl -fsSL -o "$tmp/a.tgz" "$url"; else wget -qO "$tmp/a.tgz" "$url"; fi || {
    rm -rf "$tmp"; echo "Could not download the shared build cache ($url): this worktree builds on its own." >&2; return 0; }
  got=$( (sha256sum "$tmp/a.tgz" 2>/dev/null || shasum -a 256 "$tmp/a.tgz") | cut -d' ' -f1)
  if [ "$got" = "$sum" ] && tar -xzf "$tmp/a.tgz" -C "$tmp" "$name/sccache" && mkdir -p "${bc_bin%/*}"; then
    mv "$tmp/$name/sccache" "$bc_bin.$$" && mv "$bc_bin.$$" "$bc_bin" && echo "Installed the shared build cache: $bc_bin"
  else echo "The shared build cache did not install (sha256 $got): this worktree builds on its own." >&2; fi
  rm -rf "$tmp"; return 0
}
# One server (on a socket of its own, so another sccache of yours is left alone), one cache and one size limit for
# every worktree. A RUSTC_WRAPPER you set yourself wins; HERDR_PROJECTS_NO_CACHE=1 turns the cache off.
bc_use() {
  [ -x "$bc_bin" ] && [ -z "$RUSTC_WRAPPER" ] && [ -z "$HERDR_PROJECTS_NO_CACHE" ] || return 0
  RUSTC_WRAPPER="$bc_bin"; SCCACHE_DIR="$bc_dir/sccache"; SCCACHE_SERVER_UDS="$bc_dir/sccache.sock"
  SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-2G}"; SCCACHE_CLIENT_SIDE=1 # compiles in the build's own process, not the server's
  export RUSTC_WRAPPER SCCACHE_DIR SCCACHE_SERVER_UDS SCCACHE_CACHE_SIZE SCCACHE_CLIENT_SIDE
  bc_serve || { unset RUSTC_WRAPPER; echo "The shared build cache did not start: this build runs on its own." >&2; }
}
# The server is started here, before cargo, never by rustc's first call: worktrees set up at the same moment would
# each start one, and every start that loses the race to bind the socket fails its compile ("Server startup failed:
# File exists"). A start that loses here is retried, and finds the winner's server running.
bc_serve() {
  for bc_try in 1 2 3 4 5; do
    "$bc_bin" --start-server >/dev/null 2>&1 && return 0
    sleep 1
  done
  return 1
}
case "$0" in *build-cache.sh) bc_install ;; *) bc_use ;; esac
