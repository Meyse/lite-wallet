#!/bin/sh
set -eu
# Only command names and paths are arguments. Secrets never cross this wrapper.
case "${1:-}" in
  setup|status|preflight|fill|unlock|remove|self-test) ;;
  *) echo 'Usage: scripts/test-wallet-keychain.sh setup|status|preflight|fill|unlock|remove|self-test' >&2; exit 2 ;;
esac
[ "$#" -eq 1 ] || { echo 'This helper accepts one command and no password arguments.' >&2; exit 2; }
[ "$(uname -s)" = Darwin ] || { echo 'This helper requires macOS.' >&2; exit 2; }
repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
build_dir="$repo_dir/src-tauri/target/test-wallet-keychain"
source_file="$repo_dir/scripts/test-wallet-keychain/main.swift"
binary_file="$build_dir/test-wallet-keychain"
umask 077
mkdir -p "$build_dir"
if [ ! -x "$binary_file" ] || [ "$source_file" -nt "$binary_file" ]; then
  build_file=$(mktemp "$build_dir/build.XXXXXX")
  trap 'rm -f "$build_file"' EXIT HUP INT TERM
  /usr/bin/xcrun swiftc -module-cache-path "$build_dir/module-cache" "$source_file" -o "$build_file"
  chmod 700 "$build_file"
  mv "$build_file" "$binary_file"
  trap - EXIT HUP INT TERM
fi
exec "$binary_file" "$1" "$repo_dir"
