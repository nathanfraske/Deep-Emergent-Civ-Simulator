#!/usr/bin/env python3
"""Focused tests for the declarative gate runner and client parity."""

from __future__ import annotations

import copy
import contextlib
import dataclasses
import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import gate_runner


ROOT = Path(__file__).resolve().parent.parent


def valid_data() -> dict[str, object]:
    gate = {
        "id": "test.first",
        "order": 10,
        "description": "synthetic gate",
        "tiers": ["pr"],
        "phase": "provenance",
        "command": ["{python}", "scripts/check.py"],
        "self_test": ["{python}", "scripts/check.py", "--self-test"],
        "timeout_seconds": 10,
        "cache": "content-hash",
        "inputs": ["scripts/check.py"],
        "path_triggers": ["scripts/check.py"],
    }
    return {"inventory": {"schema": 1, "tiers": ["pr"]}, "gate": [gate]}


class InventoryValidationTests(unittest.TestCase):
    def assert_invalid(self, data: dict[str, object], message: str) -> None:
        with self.assertRaisesRegex(gate_runner.InventoryError, message):
            gate_runner.parse_inventory(data, Path("synthetic-gates.toml"))

    def test_current_inventory_is_complete(self) -> None:
        inventory = gate_runner.load_inventory()
        self.assertGreater(len(inventory.gates), 20)
        self.assertIn("canonical", inventory.tiers)
        self.assertIn("pr", inventory.tiers)
        self.assertIn("full", inventory.tiers)
        self.assertIn("nightly", inventory.tiers)
        self.assertIn("legacy", inventory.tiers)
        for gate in inventory.gates:
            self.assertTrue(gate.inputs)
            self.assertTrue(gate.path_triggers)
            self.assertTrue(gate.self_test or gate.no_self_test_reason)

    def test_missing_command_fails(self) -> None:
        data = valid_data()
        del data["gate"][0]["command"]  # type: ignore[index]
        self.assert_invalid(data, "missing command")

    def test_duplicate_id_fails(self) -> None:
        data = valid_data()
        duplicate = copy.deepcopy(data["gate"][0])  # type: ignore[index]
        duplicate["order"] = 20
        data["gate"].append(duplicate)  # type: ignore[union-attr]
        self.assert_invalid(data, "duplicate gate id")

    def test_order_drift_fails(self) -> None:
        data = valid_data()
        second = copy.deepcopy(data["gate"][0])  # type: ignore[index]
        second["id"] = "test.second"
        second["order"] = 5
        data["gate"].append(second)  # type: ignore[union-attr]
        self.assert_invalid(data, "gate order drift")

    def test_shell_string_fails(self) -> None:
        data = valid_data()
        data["gate"][0]["command"] = "python scripts/check.py"  # type: ignore[index]
        self.assert_invalid(data, "argument array")


class InputClosureTests(unittest.TestCase):
    def make_root(self) -> tuple[tempfile.TemporaryDirectory[str], Path, gate_runner.Gate]:
        temporary = tempfile.TemporaryDirectory(prefix="gate-runner-test-")
        root = Path(temporary.name)
        scripts = root / "scripts"
        scripts.mkdir()
        (scripts / "check.py").write_text("print(1)\n", encoding="utf-8")
        (scripts / "gate_runner.py").write_text("runner-A\n", encoding="utf-8")
        (scripts / "gates.toml").write_text("inventory-A\n", encoding="utf-8")
        inventory = gate_runner.parse_inventory(valid_data(), scripts / "gates.toml")
        return temporary, root, inventory.gates[0]

    def test_runner_and_same_size_content_are_hashed_from_bytes(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        baseline, _ = gate_runner.input_hash(gate, root)

        (root / "scripts" / "gate_runner.py").write_text(
            "runner-B\n", encoding="utf-8"
        )
        runner_changed, _ = gate_runner.input_hash(gate, root)
        self.assertNotEqual(baseline, runner_changed)

        (root / "scripts" / "gate_runner.py").write_text(
            "runner-A\n", encoding="utf-8"
        )
        checked = root / "scripts" / "check.py"
        original_mtime = checked.stat().st_mtime_ns
        checked.write_text("print(2)\n", encoding="utf-8")
        os.utime(checked, ns=(original_mtime, original_mtime))
        content_changed, _ = gate_runner.input_hash(gate, root)
        self.assertNotEqual(baseline, content_changed)

    def test_empty_and_parent_globs_fail_closed(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        with self.assertRaisesRegex(gate_runner.InventoryError, "matched no path"):
            gate_runner.input_hash(
                dataclasses.replace(gate, inputs=("missing/**",)), root
            )
        with self.assertRaisesRegex(gate_runner.InventoryError, "escapes"):
            gate_runner.input_hash(
                dataclasses.replace(gate, inputs=("../outside.txt",)), root
            )

    def test_links_fail_closed_when_supported(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        link = root / "link.txt"
        try:
            link.symlink_to(root / "scripts" / "check.py")
        except (OSError, NotImplementedError) as error:
            self.skipTest(f"symbolic links unavailable: {error}")
        with self.assertRaisesRegex(gate_runner.InventoryError, "link or reparse"):
            gate_runner.input_hash(dataclasses.replace(gate, inputs=("link.txt",)), root)

    def test_execution_rejects_input_races_and_invalid_utf8(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        manifest = root / "scripts" / "gates.toml"
        output = io.StringIO()
        with mock.patch.object(gate_runner, "ROOT", root), contextlib.redirect_stdout(
            output
        ), contextlib.redirect_stderr(output):
            raced = gate_runner.execute_gate(
                gate,
                (
                    sys.executable,
                    "-c",
                    "from pathlib import Path; Path('scripts/check.py').write_text('print(9)\\n')",
                ),
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertFalse(raced)
        self.assertIn("changed during execution", output.getvalue())

        output = io.StringIO()
        with mock.patch.object(gate_runner, "ROOT", root), contextlib.redirect_stdout(
            output
        ), contextlib.redirect_stderr(output):
            invalid_utf8 = gate_runner.execute_gate(
                gate,
                (
                    sys.executable,
                    "-c",
                    "import sys; sys.stdout.buffer.write(b'\\xff')",
                ),
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertFalse(invalid_utf8)
        self.assertIn("not valid UTF-8", output.getvalue())


class ClientParityTests(unittest.TestCase):
    def run_client(self, command: list[str]) -> list[str]:
        result = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return [line for line in result.stdout.splitlines() if line]

    def authority_ids(self, tier: str) -> list[str]:
        return self.run_client(
            [
                sys.executable,
                "scripts/gate_runner.py",
                "list",
                "--tier",
                tier,
                "--ids-only",
            ]
        )

    def require_bash(self) -> str:
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest(
                "bash is unavailable; tracked hook execution is platform-specific"
            )
        return bash

    def test_pre_push_list_mode_matches_the_authority(self) -> None:
        bash = self.require_bash()
        expected = self.authority_ids("pr")
        explicit = self.run_client(
            [bash, "scripts/githooks/pre-push", "--list-gates", "pr"]
        )
        default = self.run_client(
            [bash, "scripts/githooks/pre-push", "--list-gates"]
        )
        self.assertEqual(expected, explicit)
        self.assertEqual(expected, default)

    def test_pre_push_preserves_git_arguments_and_stdin(self) -> None:
        bash = self.require_bash()
        with tempfile.TemporaryDirectory(prefix="pre-push-client-test-") as temporary:
            root = Path(temporary)
            hook_dir = root / "scripts" / "githooks"
            hook_dir.mkdir(parents=True)
            shutil.copy2(
                ROOT / "scripts" / "githooks" / "pre-push",
                hook_dir / "pre-push",
            )

            (root / "scripts" / "gate_runner.py").write_text(
                """from pathlib import Path
import os
import sys

Path(os.environ["GATE_ARGS_LOG"]).write_text("\\n".join(sys.argv[1:]), encoding="utf-8")
Path(os.environ["GATE_STDIN_LOG"]).write_text(sys.stdin.read(), encoding="utf-8")
raise SystemExit(int(os.environ.get("GATE_EXIT", "0")))
""",
                encoding="utf-8",
            )
            stone_hook = root / "scripts" / "stone0-pre-push-hook.sh"
            stone_hook.write_text(
                """#!/usr/bin/env bash
set -euo pipefail
printf '%s\\n' "$@" > "$STONE_ARGS_LOG"
cat > "$STONE_STDIN_LOG"
""",
                encoding="utf-8",
            )
            stone_hook.chmod(0o755)

            logs = {
                "GATE_ARGS_LOG": root / "gate-args.txt",
                "GATE_STDIN_LOG": root / "gate-stdin.txt",
                "STONE_ARGS_LOG": root / "stone-args.txt",
                "STONE_STDIN_LOG": root / "stone-stdin.txt",
            }
            environment = os.environ.copy()
            environment.update(
                {name: path.as_posix() for name, path in logs.items()}
            )
            ref_update = (
                "refs/heads/topic "
                + "1" * 40
                + " refs/heads/topic "
                + "0" * 40
                + "\n"
            )
            result = subprocess.run(
                [bash, "scripts/githooks/pre-push", "origin", "ssh://example/repo"],
                cwd=root,
                input=ref_update,
                capture_output=True,
                text=True,
                check=False,
                env=environment,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertEqual(
                logs["GATE_ARGS_LOG"].read_text(encoding="utf-8"),
                "run\n--tier\npr",
            )
            self.assertEqual(logs["GATE_STDIN_LOG"].read_text(encoding="utf-8"), "")
            self.assertEqual(
                logs["STONE_ARGS_LOG"].read_text(encoding="utf-8").splitlines(),
                ["origin", "ssh://example/repo"],
            )
            self.assertEqual(
                logs["STONE_STDIN_LOG"].read_text(encoding="utf-8"), ref_update
            )

            logs["STONE_ARGS_LOG"].unlink()
            logs["STONE_STDIN_LOG"].unlink()
            environment["GATE_EXIT"] = "7"
            blocked = subprocess.run(
                [bash, "scripts/githooks/pre-push", "origin", "ssh://example/repo"],
                cwd=root,
                input=ref_update,
                capture_output=True,
                text=True,
                check=False,
                env=environment,
            )
            self.assertEqual(blocked.returncode, 7, blocked.stdout + blocked.stderr)
            self.assertFalse(logs["STONE_ARGS_LOG"].exists())
            self.assertFalse(logs["STONE_STDIN_LOG"].exists())

    def test_static_clients_point_to_the_declarative_authority(self) -> None:
        ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("python3 scripts/gate_runner.py --self-test", ci)
        self.assertIn(
            "python3 scripts/gate_runner.py list --tier canonical --ids-only", ci
        )
        self.assertIn("run: just gates-list pr", ci)
        self.assertIn("run: just ci", ci)

        stop = (ROOT / ".claude" / "hooks" / "stop-gate.sh").read_text(
            encoding="utf-8"
        )
        for phase in ("pre", "provenance", "post"):
            self.assertIn(
                f"python3 scripts/gate_runner.py run --tier stop --phase {phase}",
                stop,
            )
        self.assertIn("python3 scripts/gate_runner.py self-tests --tier stop", stop)

        stone0 = (ROOT / "crates" / "stone0" / "src" / "lib.rs").read_text(
            encoding="utf-8"
        )
        self.assertIn('"scripts/gate_runner.py"', stone0)
        self.assertIn(
            '&["run", "--tier", "canonical", "--phase", "provenance"]',
            stone0,
        )

        makefile = (ROOT / "Makefile").read_text(encoding="utf-8")
        self.assertIn("GATE_TIER ?= pr", makefile)
        self.assertIn("GATES_LIST_CMD := $(DEV) gates-list $(GATE_TIER)", makefile)
        self.assertIn("GATES_LIST_CMD := just gates-list $(GATE_TIER)", makefile)
        self.assertIn("\ngates-list:\n\t@$(GATES_LIST_CMD)\n", makefile)

        powershell = (ROOT / "scripts" / "dev.ps1").read_text(encoding="utf-8")
        self.assertIn('"gates-list"    = "just gates-list"', powershell)
        self.assertIn('"gates-run"     = "just gates-run"', powershell)
        self.assertIn(
            '"gates-self-tests" = "just gates-self-tests"', powershell
        )

        justfile = (ROOT / "justfile").read_text(encoding="utf-8")
        self.assertIn(
            "python3 scripts/gate_runner.py list --tier {{tier}} --ids-only",
            justfile,
        )

    def test_just_and_local_ci_print_the_same_pr_ids(self) -> None:
        if shutil.which("just") is None:
            self.skipTest("just is unavailable; executable Just/local-CI parity skipped")
        bash = self.require_bash()
        bash_python = subprocess.run(
            [bash, "-lc", "python3 -c 'import tomllib' >/dev/null 2>&1"],
            cwd=ROOT,
            check=False,
        )
        if bash_python.returncode != 0:
            self.skipTest(
                "bash cannot execute Python 3.11 as python3; Linux local-CI client skipped"
            )
        authority_ids = self.authority_ids("pr")
        just_ids = self.run_client(["just", "gates-list", "pr"])
        local_ci_ids = self.run_client(
            [bash, "scripts/ci_local.sh", "--list-gates", "pr"]
        )
        self.assertEqual(authority_ids, just_ids)
        self.assertEqual(just_ids, local_ci_ids)
        self.assertIn("canonical.planet-boundary", just_ids)


if __name__ == "__main__":
    unittest.main()
