#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

set -euo pipefail

server_rel="${1:?usage: run-mcp.sh <repo-relative-server.py>}"
root="$(git rev-parse --show-toplevel)"
server="$root/$server_rel"
repo_root="$root"

case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    server="$(cygpath -w "$server")"
    repo_root="$(cygpath -w "$root")"
    ;;
esac

export REPO_ROOT="$repo_root"
if python3 -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)' >/dev/null 2>&1; then
  exec python3 "$server"
fi
if python -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 11) else 1)' >/dev/null 2>&1; then
  exec python "$server"
fi

echo "MCP launcher: Python 3.11 or newer is required." >&2
exit 1
