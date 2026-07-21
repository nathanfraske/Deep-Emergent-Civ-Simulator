#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License"); see LICENSE.
#
# The civsim-harness MCP server: a test-and-visual harness for the simulation, so the
# agent can render the world, read its deterministic stats, run the test suite, and do
# visual-regression checks as callable tools rather than shelling out by hand. A small
# stdio server speaking newline-delimited JSON-RPC 2.0, in the same shape as the
# projectops server. It depends only on the Python standard library plus Pillow (already
# used in this project for image work); if Pillow is missing, the image tools report that
# and the text tools still work.
#
# Tools:
#   render        render a living-world frame (overview or superfine) and return the PNG.
#   genesis_stats run the world-genesis sequence and return its deterministic summary.
#   test          run the cargo test suite (optionally filtered) and return pass/fail.
#   visual_diff   render a frame and compare it to a golden PNG (a visual regression test).

import base64
import io
import json
import os
import subprocess
import sys
import tempfile

try:
    from PIL import Image  # Pillow
except ImportError:  # pragma: no cover
    Image = None

ROOT = os.environ.get("REPO_ROOT") or os.path.dirname(
    os.path.dirname(os.path.abspath(__file__))
)
CARGO = os.path.join(os.path.expanduser("~"), ".cargo", "bin", "cargo")
if not os.path.exists(CARGO):
    CARGO = "cargo"


def _viewer_bin():
    """The release viewer binary, built once if it is not present."""
    binary = "civsim-viewer.exe" if os.name == "nt" else "civsim-viewer"
    path = os.path.join(ROOT, "target", "release", binary)
    if not os.path.exists(path):
        subprocess.run(
            [CARGO, "build", "--release", "-q", "-p", "civsim-viewer"],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
    return path


def _ppm_to_png_b64(ppm_path):
    if Image is None:
        raise RuntimeError("Pillow is not installed; the image tools need it")
    img = Image.open(ppm_path)
    out = io.BytesIO()
    img.save(out, format="PNG")
    return base64.b64encode(out.getvalue()).decode("ascii"), img.size


# --- tools ---------------------------------------------------------------------


def tool_render(args):
    seed = str(args.get("seed", "0xEA27"))
    width = str(int(args.get("width", 256)))
    height = str(int(args.get("height", 192)))
    mode = args.get("mode", "overview")
    if mode not in ("overview", "superfine"):
        raise ValueError("mode must be 'overview' or 'superfine'")
    with tempfile.TemporaryDirectory() as tmp:
        ppm = os.path.join(tmp, "frame.ppm")
        proc = subprocess.run(
            [_viewer_bin(), "--render", ppm, mode, seed, width, height],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if not os.path.exists(ppm):
            raise RuntimeError(f"render failed: {proc.stderr.strip()}")
        data, size = _ppm_to_png_b64(ppm)
    return {
        "__image__": data,
        "caption": f"{mode} of seed {seed} at {width}x{height} ({size[0]}x{size[1]}px)",
    }


def tool_genesis_stats(args):
    seed = str(args.get("seed", "0xEA27"))
    width = str(int(args.get("width", 256)))
    height = str(int(args.get("height", 192)))
    proc = subprocess.run(
        [_viewer_bin(), "--stats", seed, width, height],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    line = proc.stdout.strip().splitlines()[-1] if proc.stdout.strip() else "{}"
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return {"error": "could not parse stats", "raw": proc.stdout, "stderr": proc.stderr}


def tool_test(args):
    cmd = [CARGO, "test"]
    pkg = args.get("package")
    if pkg:
        cmd += ["-p", str(pkg)]
    else:
        cmd += ["--workspace"]
    if args.get("filter"):
        cmd.append(str(args["filter"]))
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    out = proc.stdout + proc.stderr
    passed = sum(
        int(w) for line in out.splitlines() if "test result:" in line
        for w in [line.split("test result:")[1].split("passed")[0].strip().split()[-1]]
        if w.isdigit()
    )
    failed = sum(
        int(seg.split("failed")[0].strip().split()[-1])
        for line in out.splitlines()
        if "test result:" in line
        for seg in [line.split(";")[1]]
        if "failed" in seg and seg.split("failed")[0].strip().split()[-1].isdigit()
    )
    return {
        "ok": proc.returncode == 0,
        "passed": passed,
        "failed": failed,
        "tail": "\n".join(out.splitlines()[-12:]),
    }


def tool_visual_diff(args):
    if Image is None:
        raise RuntimeError("Pillow is not installed; visual_diff needs it")
    golden = args.get("golden")
    if not golden:
        raise ValueError("visual_diff needs a 'golden' PNG path")
    golden = golden if os.path.isabs(golden) else os.path.join(ROOT, golden)
    seed = str(args.get("seed", "0xEA27"))
    width = str(int(args.get("width", 256)))
    height = str(int(args.get("height", 192)))
    mode = args.get("mode", "overview")
    with tempfile.TemporaryDirectory() as tmp:
        ppm = os.path.join(tmp, "frame.ppm")
        subprocess.run(
            [_viewer_bin(), "--render", ppm, mode, seed, width, height],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        if not os.path.exists(ppm):
            raise RuntimeError("render failed")
        cur = Image.open(ppm).convert("RGB")
        if not os.path.exists(golden):
            cur.save(golden, format="PNG")
            return {"created_golden": golden, "size": cur.size, "note": "no golden existed; saved the current frame as the golden"}
        gold = Image.open(golden).convert("RGB")
        if cur.size != gold.size:
            return {"match": False, "reason": "size differs", "current": cur.size, "golden": gold.size}
        cb = cur.tobytes()
        gb = gold.tobytes()
        diff = sum(1 for a, b in zip(cb, gb) if a != b)
        frac = diff / max(1, len(cb))
        return {
            "match": frac == 0.0,
            "mismatch_fraction": round(frac, 6),
            "size": cur.size,
            "golden": golden,
        }


TOOLS = {
    "render": (
        tool_render,
        "Render a living-world frame (mode 'overview' for the whole colour map or "
        "'superfine' for the individual organisms) from a seed and world size, and "
        "return it as a PNG image.",
        {
            "type": "object",
            "properties": {
                "seed": {"type": "string", "description": "world seed, decimal or 0x hex"},
                "width": {"type": "integer"},
                "height": {"type": "integer"},
                "mode": {"type": "string", "enum": ["overview", "superfine"]},
            },
        },
    ),
    "genesis_stats": (
        tool_genesis_stats,
        "Run the world-genesis sequence and return its deterministic summary: regions, "
        "species, alive, daughters, extinctions, and the state hash.",
        {
            "type": "object",
            "properties": {
                "seed": {"type": "string"},
                "width": {"type": "integer"},
                "height": {"type": "integer"},
            },
        },
    ),
    "test": (
        tool_test,
        "Run the cargo test suite (the whole workspace, or a package, optionally "
        "filtered) and return pass/fail counts with the tail of the output.",
        {
            "type": "object",
            "properties": {
                "package": {"type": "string", "description": "e.g. civsim-sim"},
                "filter": {"type": "string", "description": "a test-name filter"},
            },
        },
    ),
    "visual_diff": (
        tool_visual_diff,
        "Render a frame and compare it to a golden PNG (a visual regression test). "
        "Returns the mismatch fraction; saves the frame as the golden if none exists.",
        {
            "type": "object",
            "properties": {
                "golden": {"type": "string", "description": "path to the golden PNG"},
                "seed": {"type": "string"},
                "width": {"type": "integer"},
                "height": {"type": "integer"},
                "mode": {"type": "string", "enum": ["overview", "superfine"]},
            },
            "required": ["golden"],
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


def _content_for(result):
    """An image result carries an __image__ base64 payload; everything else is JSON text."""
    if isinstance(result, dict) and "__image__" in result:
        content = [{"type": "image", "data": result["__image__"], "mimeType": "image/png"}]
        if result.get("caption"):
            content.append({"type": "text", "text": result["caption"]})
        return content
    return [{"type": "text", "text": json.dumps(result, indent=2)}]


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
                "serverInfo": {"name": "civsim-harness", "version": "0.1.0"},
            },
        }
    if method == "tools/list":
        return {"jsonrpc": "2.0", "id": rid, "result": {"tools": tools_list()}}
    if method == "tools/call":
        params = req.get("params", {})
        name = params.get("name")
        args = params.get("arguments", {})
        try:
            result = dispatch(name, args)
            return {"jsonrpc": "2.0", "id": rid, "result": {"content": _content_for(result)}}
        except Exception as exc:  # noqa: BLE001
            return {
                "jsonrpc": "2.0",
                "id": rid,
                "result": {"content": [{"type": "text", "text": f"error: {exc}"}], "isError": True},
            }
    if method == "ping":
        return {"jsonrpc": "2.0", "id": rid, "result": {}}
    if method in ("notifications/initialized", "initialized"):
        return None
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
