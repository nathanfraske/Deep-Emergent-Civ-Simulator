#!/usr/bin/env bash
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# Fail-closed PreToolUse guard for panel-shaped Workflow calls. The Python
# implementation validates a real two-stage smoke gate rather than accepting
# acknowledgment words alone. Its self-test is also the durable hook regression
# matrix used by setup and review.

set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec python3 "$ROOT/.claude/hooks/workflow_panel_guard.py" "$@"
