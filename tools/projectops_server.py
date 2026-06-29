#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# The projectops MCP server (AGENTIC_ADDENDUM.md section 3). A small stdio server
# that turns the verification suite and the project's structured data into callable
# tools, so the hooks and panels consume structured results rather than re-deriving
# them from raw greps. It speaks newline-delimited JSON-RPC 2.0 over stdin and
# stdout and depends only on the Python standard library (tomllib is built in on
# Python 3.11+), so it runs with no install step.
#
# Tools:
#   verify              run scripts/verify.sh and return structured JSON.
#   backlog             parse TODOS.md and audit Section 3 into the open-items list.
#   reserved            parse calibration/reserved.toml into the review queue.
#   consolidation_check given an item id, confirm a resolution is complete.

import json
import os
import re
import subprocess
import sys

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None

ROOT = os.environ.get("REPO_ROOT") or os.path.dirname(
    os.path.dirname(os.path.abspath(__file__))
)


def _read(path):
    try:
        with open(path, encoding="utf-8") as fh:
            return fh.read()
    except OSError:
        return ""


def design_text():
    return _read(os.path.join(ROOT, "docs", "design.md"))


def audit_text():
    return _read(os.path.join(ROOT, "docs", "audit.md"))


def _section(text, start_pat, end_pat):
    """Return the slice of text from a heading matching start_pat up to the next
    heading matching end_pat (or end of text)."""
    s = re.search(start_pat, text, re.M)
    if not s:
        return ""
    rest = text[s.start():]
    e = re.search(end_pat, rest[1:], re.M)
    return rest[: e.start() + 1] if e else rest


# --- tools ---------------------------------------------------------------------


def tool_verify(_args):
    script = os.path.join(ROOT, "scripts", "verify.sh")
    proc = subprocess.run(
        ["bash", script, "--json"], capture_output=True, text=True
    )
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"ok": False, "error": (proc.stdout + proc.stderr).strip()}


def tool_backlog(_args):
    items = []
    for line in _read(os.path.join(ROOT, "TODOS.md")).splitlines():
        m = re.match(r"^- \*\*(R-[A-Z0-9-]+)\.\*\*\s*(.*)$", line)
        if m:
            items.append({"id": m.group(1), "summary": m.group(2).strip()})
    open_count = len(
        re.findall(r"^- \*\*R-", audit_text(), re.M)
    )
    return {
        "open_count_audit": open_count,
        "todos_open_items": len(items),
        "items": items,
    }


def tool_reserved(_args):
    path = os.path.join(ROOT, "calibration", "reserved.toml")
    if tomllib is None:
        return {"error": "tomllib unavailable; Python 3.11+ required"}
    try:
        with open(path, "rb") as fh:
            data = tomllib.load(fh)
    except (OSError, ValueError) as exc:
        return {"error": str(exc)}
    queue = []
    for e in data.get("reserved", []):
        is_set = e.get("status") == "set" and str(e.get("value", "")).strip() != ""
        queue.append(
            {
                "id": e.get("id"),
                "basis": e.get("basis"),
                "status": e.get("status"),
                "value": e.get("value", ""),
                "unit": e.get("unit", ""),
                "source": e.get("source"),
                "is_set": is_set,
            }
        )
    reserved_ids = [q["id"] for q in queue if not q["is_set"]]
    return {
        "total": len(queue),
        "reserved_count": len(reserved_ids),
        "set_count": len(queue) - len(reserved_ids),
        "reserved_ids": reserved_ids,
        "queue": queue,
    }


def _near(text, item, words, window=60):
    """True if any of `words` appears within `window` characters of an occurrence
    of `item`. The maintained documents are not perfectly uniform in how they mark
    a resolution (some items are Section 3 bullets, some are subsections), so the
    check is proximity-based rather than tied to one exact phrasing."""
    low = [w.lower() for w in words]
    for m in re.finditer(re.escape(item), text):
        seg = text[max(0, m.start() - window) : m.end() + window].lower()
        if any(w in seg for w in low):
            return True
    return False


def tool_consolidation_check(args):
    item = (args or {}).get("item_id", "")
    if not re.match(r"^R-[A-Z0-9-]+$", item or ""):
        return {"error": "item_id must look like R-XXXX"}
    design = design_text()
    audit = audit_text()

    # An open item carries a "> Needs research, item R-XXX" flag at its site.
    flag_present = bool(
        re.search(r"> Needs research, item " + re.escape(item) + r"\b", design)
    )
    # An open item is counted as a "- **R-XXX." bullet in the audit backlog.
    open_bullet = bool(re.search(r"^- \*\*" + re.escape(item) + r"\.", audit, re.M))
    # A resolved item is marked resolved or consolidated near its id in the audit.
    resolved_marker = _near(audit, item, ["resolved", "consolidated"], window=45)
    # A resolved item points at a Part 62 record near its id.
    record_pointer = _near(
        audit, item, ["record in part 62", "record 62", "part 62"], window=160
    )
    part63 = _section(design, r"^## Part 63:", r"^## Part \d+:")
    bibliography_present = item in part63

    complete = resolved_marker and (not flag_present) and (not open_bullet)
    return {
        "item_id": item,
        "flag_replaced": not flag_present,
        "marked_resolved_in_audit": resolved_marker,
        "record_pointer_present": record_pointer,
        "bibliography_in_part_63": bibliography_present,
        "still_open_in_backlog": open_bullet,
        "resolution_complete": complete,
    }


TOOLS = {
    "verify": (
        tool_verify,
        "Run the verification suite over the maintained documents and return "
        "structured pass-or-fail results.",
        {"type": "object", "properties": {}},
    ),
    "backlog": (
        tool_backlog,
        "Parse TODOS.md and the audit backlog into the open research items with "
        "their summaries and the running open count.",
        {"type": "object", "properties": {}},
    ),
    "reserved": (
        tool_reserved,
        "Parse calibration/reserved.toml into the reserved-values review queue: "
        "each value with its id, basis, status, value, and source.",
        {"type": "object", "properties": {}},
    ),
    "consolidation_check": (
        tool_consolidation_check,
        "Given a research item id, confirm its resolution is complete: flag "
        "replaced, record in Part 62, bibliography in Part 63, backlog bullet "
        "rewritten to resolved.",
        {
            "type": "object",
            "properties": {
                "item_id": {
                    "type": "string",
                    "description": "Research item id, for example R-EVIDENCE.",
                }
            },
            "required": ["item_id"],
        },
    ),
}


def tools_list():
    return [
        {"name": name, "description": desc, "inputSchema": schema}
        for name, (_fn, desc, schema) in TOOLS.items()
    ]


def dispatch(name, args):
    if name not in TOOLS:
        raise ValueError(f"unknown tool: {name}")
    return TOOLS[name][0](args)


# --- JSON-RPC plumbing ---------------------------------------------------------


def handle(req):
    method = req.get("method")
    rid = req.get("id")

    if method == "initialize":
        return {
            "jsonrpc": "2.0",
            "id": rid,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "projectops", "version": "0.1.0"},
            },
        }
    if method == "tools/list":
        return {"jsonrpc": "2.0", "id": rid, "result": {"tools": tools_list()}}
    if method == "tools/call":
        params = req.get("params", {}) or {}
        name = params.get("name")
        args = params.get("arguments", {}) or {}
        try:
            result = dispatch(name, args)
            return {
                "jsonrpc": "2.0",
                "id": rid,
                "result": {
                    "content": [
                        {"type": "text", "text": json.dumps(result, indent=2)}
                    ]
                },
            }
        except Exception as exc:  # surface as a tool error, not a transport error
            return {
                "jsonrpc": "2.0",
                "id": rid,
                "result": {
                    "content": [{"type": "text", "text": f"error: {exc}"}],
                    "isError": True,
                },
            }
    if method == "ping":
        return {"jsonrpc": "2.0", "id": rid, "result": {}}

    if rid is None:
        return None  # a notification; no response
    return {
        "jsonrpc": "2.0",
        "id": rid,
        "error": {"code": -32601, "message": f"method not found: {method}"},
    }


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
        except json.JSONDecodeError:
            continue
        resp = handle(req)
        if resp is not None:
            sys.stdout.write(json.dumps(resp) + "\n")
            sys.stdout.flush()


if __name__ == "__main__":
    main()
