#!/usr/bin/env python3
"""Fail-closed construction guard for load-bearing panel workflows."""

from __future__ import annotations

import json
import re
import sys
import tempfile
from pathlib import Path
from typing import Callable


PANEL_PATTERN = re.compile(
    r"audit|panel|lens|blind|adversar|confirmation[.-]?bias|framing",
    re.IGNORECASE,
)
MARKERS = (
    re.compile(r"^\s*//\s*panels-reviewed\s*$", re.MULTILINE),
    re.compile(r"^\s*//\s*input-bias-smoke-cleared\s*$", re.MULTILINE),
    re.compile(r"^\s*//\s*panel-smoke-protocol-v1\s*$", re.MULTILINE),
)
REQUIRED_TOKENS = (
    "SMOKE_SCHEMA",
    "CLEAR",
    "BLOCK",
    "OWNER_REVIEW",
    "genericized_input",
    "neutralized_notes",
    "material_defects",
    "defect_class",
    "verdict_consequence",
    "required_change",
    "substantial",
    "repeats_prior_class",
    "new_consequence",
    "materiality_disputed",
    "correction_history",
    "normalizeConsequence",
    "next_smoke_history",
    "SMOKE_HISTORY_REQUIRED",
)


REMINDER = """\
STANDING PANEL WORKFLOW BLOCKED (AGENTIC_ADDENDUM.md sections 7 through 11).
Panel-shaped workflows require all relevant standing lenses and a real
fail-closed section-11 smoke stage before panel launch. The smoke must receive
the complete construction and correction history, return a structured verdict,
genericize minor wording, surplus context, and non-load-bearing metadata into
its own neutral panel input, and reserve BLOCK for a substantial omission,
steering seam, traceability break, or lens gap that can change a substantive
verdict. A repeated defect class without a new concrete verdict consequence is
a CLEAR note. Disputed materiality returns OWNER_REVIEW and stops recursion.

Use the active templates under .claude/skills/panel/templates/ or implement the
same panel-smoke-protocol-v1 structure. Marker comments alone are insufficient.
"""


def _read_script(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def _strip_js_comments(source: str) -> str:
    """Remove JavaScript comments while preserving quoted and template text."""
    output: list[str] = []
    index = 0
    quote: str | None = None
    while index < len(source):
        current = source[index]
        following = source[index + 1] if index + 1 < len(source) else ""
        if quote is not None:
            output.append(current)
            if current == "\\" and index + 1 < len(source):
                index += 1
                output.append(source[index])
            elif current == quote:
                quote = None
            index += 1
            continue
        if current in ("'", '"', "`"):
            quote = current
            output.append(current)
            index += 1
            continue
        if current == "/" and following == "/":
            index += 2
            while index < len(source) and source[index] not in "\r\n":
                index += 1
            continue
        if current == "/" and following == "*":
            index += 2
            while index + 1 < len(source) and source[index : index + 2] != "*/":
                index += 1
            index = min(index + 2, len(source))
            continue
        output.append(current)
        index += 1
    return "".join(output)


def inspect_payload(
    raw: str, read_script: Callable[[str], str] = _read_script
) -> tuple[int, str]:
    try:
        data = json.loads(raw)
    except Exception as exc:
        return 2, f"workflow panel guard could not parse hook JSON: {exc}\n"

    if not isinstance(data, dict):
        return 2, "workflow panel guard requires a JSON object payload\n"
    tool_input = data.get("tool_input")
    if not isinstance(tool_input, dict):
        return 2, "workflow panel guard requires tool_input object\n"

    parts: list[str] = []
    for key in ("script", "name", "prompt", "message", "task_name"):
        value = tool_input.get(key)
        if isinstance(value, str):
            parts.append(value)

    script_path = tool_input.get("scriptPath")
    if script_path is not None:
        if not isinstance(script_path, str) or not script_path:
            return 2, "workflow panel guard received an invalid scriptPath\n"
        try:
            parts.append(read_script(script_path))
        except Exception as exc:
            return 2, f"workflow panel guard could not read scriptPath: {exc}\n"

    blob = "\n".join(parts)
    if not PANEL_PATTERN.search(blob):
        return 0, ""

    violations: list[str] = []
    for marker in MARKERS:
        if not marker.search(blob):
            violations.append(f"missing exact marker {marker.pattern}")
    code = _strip_js_comments(blob)
    for token in REQUIRED_TOKENS:
        if token not in code:
            violations.append(f"missing structured smoke token {token}")

    smoke_match = re.search(r"\b(?:const|let)\s+smoke\s*=\s*await\s+agent\s*\(", code)
    clear_gate = re.search(
        r"if\s*\(\s*smoke\.effective_verdict\s*!==\s*['\"]CLEAR['\"]\s*\)"
        r"\s*\{(?:(?!\bparallel\s*\().){0,2500}?"
        r"\breturn\s*\{\s*status\s*:\s*['\"]SMOKE_BLOCK['\"]",
        code,
        re.DOTALL,
    )
    panel_launch = re.search(r"\bparallel\s*\(", code)
    genericized_use = re.search(r"smoke\.genericized_input", code)

    if smoke_match is None:
        violations.append("missing awaited smoke agent result")
    if clear_gate is None:
        violations.append("missing explicit CLEAR-only launch guard")
    if panel_launch is None:
        violations.append("missing isolated panel launch")
    if genericized_use is None:
        violations.append("panel does not consume smoke.genericized_input")
    if smoke_match and clear_gate and smoke_match.start() > clear_gate.start():
        violations.append("CLEAR guard occurs before smoke result")
    if clear_gate and panel_launch and clear_gate.start() > panel_launch.start():
        violations.append("panel launch occurs before CLEAR guard")
    if genericized_use and panel_launch and genericized_use.start() > panel_launch.start():
        violations.append("genericized panel input is not prepared before launch")

    if violations:
        return 2, REMINDER + "\nConstruction defects:\n  - " + "\n  - ".join(violations) + "\n"
    return 0, ""


def _fixture_script(*, markers: bool, protocol: bool, terminating_gate: bool = True) -> str:
    marker_text = (
        "// panels-reviewed\n"
        "// input-bias-smoke-cleared\n"
        "// panel-smoke-protocol-v1\n"
        if markers
        else ""
    )
    if not protocol:
        return marker_text + "const name = 'audit panel'; return { ok: true }\n"
    tokens = ", ".join(f"{token}: true" for token in REQUIRED_TOKENS)
    gate = (
        "if (smoke.effective_verdict !== 'CLEAR') "
        "{ return { status: 'SMOKE_BLOCK' } }\n"
        if terminating_gate
        else "if (smoke.effective_verdict !== 'CLEAR') console.log('ignored')\n"
    )
    return (
        marker_text
        + f"const SMOKE_SCHEMA = {{ {tokens} }}\n"
        + "const correction_history = []\n"
        + "const normalizeConsequence = (value) => value\n"
        + "const next_smoke_history = correction_history\n"
        + "const smoke = await agent('audit smoke', { schema: SMOKE_SCHEMA })\n"
        + "smoke.effective_verdict = smoke.verdict\n"
        + gate
        + "const packet = smoke.genericized_input\n"
        + "const results = await parallel([() => agent(packet)])\n"
        + "return results\n"
    )


def self_test() -> int:
    cases: list[tuple[str, str, int]] = [
        (
            "non-panel",
            json.dumps({"tool_input": {"script": "return await migrate(args)"}}),
            0,
        ),
        (
            "unacknowledged-panel",
            json.dumps({"tool_input": {"script": "const name = 'audit panel'"}}),
            2,
        ),
        (
            "structured-panel",
            json.dumps({"tool_input": {"script": _fixture_script(markers=True, protocol=True)}}),
            0,
        ),
        (
            "marker-only-panel",
            json.dumps({"tool_input": {"script": _fixture_script(markers=True, protocol=False)}}),
            2,
        ),
        (
            "no-op-clear-check",
            json.dumps(
                {
                    "tool_input": {
                        "script": _fixture_script(
                            markers=True,
                            protocol=True,
                            terminating_gate=False,
                        )
                    }
                }
            ),
            2,
        ),
        ("malformed-json", "{", 2),
        (
            "missing-script-path",
            json.dumps({"tool_input": {"name": "audit", "scriptPath": "missing.js"}}),
            2,
        ),
    ]

    with tempfile.TemporaryDirectory() as directory:
        script = Path(directory) / "panel.js"
        script.write_text(_fixture_script(markers=True, protocol=True), encoding="utf-8")
        cases.append(
            (
                "structured-script-path",
                json.dumps({"tool_input": {"name": "audit", "scriptPath": str(script)}}),
                0,
            )
        )
        root = Path(__file__).resolve().parents[2]
        for template_name in ("lens-audit.js", "framing-panel.js"):
            template = (
                root / ".claude" / "skills" / "panel" / "templates" / template_name
            )
            cases.append(
                (
                    f"active-{template_name}",
                    json.dumps(
                        {
                            "tool_input": {
                                "name": f"panel {template_name}",
                                "scriptPath": str(template),
                            }
                        }
                    ),
                    0,
                )
            )

        failed = False
        for name, payload, expected in cases:
            status, message = inspect_payload(payload)
            if status != expected:
                failed = True
                print(
                    f"workflow panel guard self-test {name}: expected {expected}, "
                    f"received {status}: {message}",
                    file=sys.stderr,
                )
        if failed:
            return 1

    print("workflow panel guard self-test: PASS (10 cases)")
    return 0


def main() -> int:
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        return self_test()
    if len(sys.argv) != 1:
        print("usage: workflow_panel_guard.py [--self-test]", file=sys.stderr)
        return 2
    status, message = inspect_payload(sys.stdin.read())
    if message:
        sys.stderr.write(message)
    return status


if __name__ == "__main__":
    raise SystemExit(main())
