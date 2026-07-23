#!/usr/bin/env python3
"""Focused tests for the declarative gate runner and client parity."""

from __future__ import annotations

import copy
import contextlib
import dataclasses
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import types
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


def mandatory_gate_blocks() -> str:
    manifest = (ROOT / "scripts" / "gates.toml").read_text(encoding="utf-8")
    selected = []
    for raw in manifest.split("[[gate]]")[1:]:
        block = "[[gate]]" + raw
        if any(
            f'id = "{gate.gate_id}"' in block
            for gate in gate_runner.MANDATORY_AUTHORITY_GATES
        ):
            selected.append(block.rstrip())
    if len(selected) != len(gate_runner.MANDATORY_AUTHORITY_GATES):
        raise AssertionError("production manifest is missing a mandatory authority block")
    return "\n\n".join(selected) + "\n"


class InventoryValidationTests(unittest.TestCase):
    def assert_invalid(self, data: dict[str, object], message: str) -> None:
        with self.assertRaisesRegex(gate_runner.InventoryError, message):
            gate_runner.parse_inventory(
                data, Path("synthetic-gates.toml"), enforce_mandatory=False
            )

    def test_strict_mode_requires_the_mandatory_authority_gates(self) -> None:
        with self.assertRaisesRegex(
            gate_runner.InventoryError, "mandatory authority gate"
        ):
            gate_runner.parse_inventory(valid_data(), Path("synthetic-gates.toml"))

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
        by_id = {gate.gate_id: gate for gate in inventory.gates}
        self.assertIn(
            "crates/*/data/*/manifest.toml",
            by_id["canonical.source-registry"].inputs,
        )
        self.assertIn("crates/planet", by_id["canonical.ledger-inventory"].inputs)
        self.assertIn("scripts/*.py", by_id["canonical.ledger-inventory"].inputs)
        for gate_id in (
            "docs.legacy-archive",
            "canonical.planet-boundary",
            "canonical.source-registry",
            "canonical.diamond-single-provider",
            "canonical.ledger-inventory",
        ):
            self.assertEqual(by_id[gate_id].cache, "never")
            self.assertTrue(by_id[gate_id].no_cache_reason)

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

    def test_phase_drift_fails_within_a_tier(self) -> None:
        data = valid_data()
        second = copy.deepcopy(data["gate"][0])  # type: ignore[index]
        second["id"] = "test.second"
        second["order"] = 20
        second["phase"] = "pre"
        data["gate"].append(second)  # type: ignore[union-attr]
        self.assert_invalid(data, "gate phase drift")

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
        inventory = gate_runner.parse_inventory(
            valid_data(), scripts / "gates.toml", enforce_mandatory=False
        )
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
        self.assertIs(raced, gate_runner.GateOutcome.OperationalFailure)
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
        self.assertIs(invalid_utf8, gate_runner.GateOutcome.OperationalFailure)
        self.assertIn("not valid UTF-8", output.getvalue())

    def test_execution_lock_rejects_linked_cache_state(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        manifest = root / "scripts" / "gates.toml"
        external_cache = root / "external-cache"
        external_cache.mkdir()
        linked_cache = root / "linked-cache"
        try:
            linked_cache.symlink_to(external_cache, target_is_directory=True)
        except (OSError, NotImplementedError) as error:
            self.skipTest(f"directory links unavailable: {error}")

        output = io.StringIO()
        with mock.patch.object(gate_runner, "ROOT", root), contextlib.redirect_stdout(
            output
        ), contextlib.redirect_stderr(output):
            outcome = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=linked_cache,
            )
        self.assertIs(outcome, gate_runner.GateOutcome.OperationalFailure)
        self.assertIn("execution lock unavailable", output.getvalue())

    def test_execute_gate_distinguishes_policy_and_operational_failures(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        manifest = root / "scripts" / "gates.toml"

        (root / "scripts" / "check.py").write_text(
            "import sys\n"
            f"print({gate_runner.LEAF_POLICY_DETECTION_MARKER!r}, file=sys.stderr)\n"
            "raise SystemExit(1)\n",
            encoding="utf-8",
        )
        with mock.patch.object(gate_runner, "ROOT", root):
            detected = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertIs(detected, gate_runner.GateOutcome.PolicyDetection)

        (root / "scripts" / "check.py").write_text(
            "raise SystemExit(1)\n", encoding="utf-8"
        )
        with mock.patch.object(gate_runner, "ROOT", root):
            unmarked = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertIs(unmarked, gate_runner.GateOutcome.OperationalFailure)

        (root / "scripts" / "check.py").write_text(
            "import sys\n"
            f"print({gate_runner.LEAF_POLICY_DETECTION_MARKER!r}, file=sys.stderr)\n"
            "raise SystemExit(2)\n",
            encoding="utf-8",
        )
        with mock.patch.object(gate_runner, "ROOT", root):
            wrong_exit = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertIs(wrong_exit, gate_runner.GateOutcome.OperationalFailure)

        missing = dataclasses.replace(gate, command=("definitely-no-such-program",))
        with mock.patch.object(gate_runner, "ROOT", root):
            unavailable = gate_runner.execute_gate(
                missing,
                missing.command,
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertIs(unavailable, gate_runner.GateOutcome.OperationalFailure)

        with mock.patch.object(gate_runner, "ROOT", root), mock.patch.object(
            gate_runner.subprocess,
            "run",
            side_effect=subprocess.TimeoutExpired("synthetic", gate.timeout_seconds),
        ):
            timed_out = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
            )
        self.assertIs(timed_out, gate_runner.GateOutcome.OperationalFailure)

    def test_only_policy_failure_emits_the_stone0_protocol_marker(self) -> None:
        inventory = gate_runner.parse_inventory(
            valid_data(), Path("synthetic-gates.toml"), enforce_mandatory=False
        )
        args = types.SimpleNamespace(
            tier="pr",
            gate_id=[],
            phase="provenance",
            dry_run=False,
            fail_fast=False,
        )
        output = io.StringIO()
        with mock.patch.object(
            gate_runner,
            "execute_gate",
            return_value=gate_runner.GateOutcome.PolicyDetection,
        ), contextlib.redirect_stderr(output):
            result = gate_runner.command_run(inventory, args)
        self.assertEqual(result, 1)
        self.assertEqual(
            output.getvalue().strip(), gate_runner.POLICY_DETECTION_MARKER
        )

        output = io.StringIO()
        with mock.patch.object(
            gate_runner,
            "execute_gate",
            return_value=gate_runner.GateOutcome.OperationalFailure,
        ), contextlib.redirect_stderr(output):
            result = gate_runner.command_run(inventory, args)
        self.assertEqual(result, 2)
        self.assertNotIn(gate_runner.POLICY_DETECTION_MARKER, output.getvalue())

    def test_success_receipts_are_content_and_command_bound(self) -> None:
        temporary, root, gate = self.make_root()
        self.addCleanup(temporary.cleanup)
        manifest = root / "scripts" / "gates.toml"
        cache_dir = root / "cache"
        counter = root / "counter.txt"
        script = root / "scripts" / "check.py"
        script.write_text(
            "from pathlib import Path\n"
            "counter = Path('counter.txt')\n"
            "value = int(counter.read_text() if counter.exists() else '0')\n"
            "counter.write_text(str(value + 1))\n",
            encoding="utf-8",
        )

        with mock.patch.object(gate_runner, "ROOT", root):
            first = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=cache_dir,
                read_cache=True,
                write_cache=True,
            )
            second_output = io.StringIO()
            with contextlib.redirect_stdout(second_output):
                second = gate_runner.execute_gate(
                    gate,
                    gate.command,
                    dry_run=False,
                    manifest_path=manifest,
                    cache_dir=cache_dir,
                    read_cache=True,
                    write_cache=True,
                )
        self.assertIs(first, gate_runner.GateOutcome.Passed)
        self.assertIs(second, gate_runner.GateOutcome.Passed)
        self.assertEqual(counter.read_text(encoding="utf-8"), "1")
        self.assertIn("[CACHED] test.first", second_output.getvalue())

        receipt_path = next(cache_dir.rglob("*.json"))
        expired = json.loads(receipt_path.read_text(encoding="utf-8"))
        expired["created_unix"] = 0
        receipt_path.write_text(json.dumps(expired), encoding="utf-8")
        with mock.patch.object(gate_runner, "ROOT", root):
            after_expiry = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=cache_dir,
                read_cache=True,
                write_cache=True,
            )
        self.assertIs(after_expiry, gate_runner.GateOutcome.Passed)
        self.assertEqual(counter.read_text(encoding="utf-8"), "2")

        script.write_text(script.read_text(encoding="utf-8") + "# changed\n", encoding="utf-8")
        changed_command = dataclasses.replace(
            gate,
            command=gate.command + ("ignored-argument",),
        )
        with mock.patch.object(gate_runner, "ROOT", root):
            after_input_change = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=cache_dir,
                read_cache=True,
                write_cache=True,
            )
            command_change = gate_runner.execute_gate(
                changed_command,
                changed_command.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=cache_dir,
                read_cache=True,
                write_cache=True,
            )
        self.assertIs(after_input_change, gate_runner.GateOutcome.Passed)
        self.assertIs(command_change, gate_runner.GateOutcome.Passed)
        self.assertEqual(counter.read_text(encoding="utf-8"), "4")

        changed_path = os.environ.get("PATH", "") + os.pathsep + str(root / "unused")
        with mock.patch.dict(os.environ, {"PATH": changed_path}):
            with mock.patch.object(gate_runner, "ROOT", root):
                environment_change = gate_runner.execute_gate(
                    gate,
                    gate.command,
                    dry_run=False,
                    manifest_path=manifest,
                    cache_dir=cache_dir,
                    read_cache=True,
                    write_cache=True,
                )
                environment_hit = gate_runner.execute_gate(
                    gate,
                    gate.command,
                    dry_run=False,
                    manifest_path=manifest,
                    cache_dir=cache_dir,
                    read_cache=True,
                    write_cache=True,
                )
        self.assertIs(environment_change, gate_runner.GateOutcome.Passed)
        self.assertIs(environment_hit, gate_runner.GateOutcome.Passed)
        self.assertEqual(counter.read_text(encoding="utf-8"), "5")

        never_cached = dataclasses.replace(gate, cache="never", no_cache_reason="test")
        with mock.patch.object(gate_runner, "ROOT", root):
            for _ in range(2):
                outcome = gate_runner.execute_gate(
                    never_cached,
                    never_cached.command,
                    dry_run=False,
                    manifest_path=manifest,
                    cache_dir=cache_dir,
                    read_cache=True,
                    write_cache=True,
                )
                self.assertIs(outcome, gate_runner.GateOutcome.Passed)
        self.assertEqual(counter.read_text(encoding="utf-8"), "7")

        secret = "synthetic-owner-override-must-not-enter-receipts"
        with mock.patch.dict(os.environ, {"STONE0_OVERRIDE": secret}):
            with mock.patch.object(gate_runner, "ROOT", root):
                for _ in range(2):
                    outcome = gate_runner.execute_gate(
                        gate,
                        gate.command,
                        dry_run=False,
                        manifest_path=manifest,
                        cache_dir=cache_dir,
                        read_cache=True,
                        write_cache=True,
                    )
                    self.assertIs(outcome, gate_runner.GateOutcome.Passed)
        self.assertEqual(counter.read_text(encoding="utf-8"), "9")
        self.assertNotIn(
            secret,
            "".join(path.read_text(encoding="utf-8") for path in cache_dir.rglob("*.json")),
        )

        receipts_before_failure = sorted(cache_dir.rglob("*.json"))
        script.write_text(
            "from pathlib import Path\n"
            "counter = Path('counter.txt')\n"
            "value = int(counter.read_text() if counter.exists() else '0')\n"
            "counter.write_text(str(value + 1))\n"
            "raise SystemExit(3)\n",
            encoding="utf-8",
        )
        with mock.patch.object(gate_runner, "ROOT", root):
            failed_once = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=cache_dir,
                read_cache=True,
                write_cache=True,
            )
            failed_twice = gate_runner.execute_gate(
                gate,
                gate.command,
                dry_run=False,
                manifest_path=manifest,
                cache_dir=cache_dir,
                read_cache=True,
                write_cache=True,
            )
        self.assertIs(failed_once, gate_runner.GateOutcome.OperationalFailure)
        self.assertIs(failed_twice, gate_runner.GateOutcome.OperationalFailure)
        self.assertEqual(counter.read_text(encoding="utf-8"), "11")
        self.assertEqual(sorted(cache_dir.rglob("*.json")), receipts_before_failure)


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

    def test_cross_process_lock_collapses_identical_content_runs(self) -> None:
        with tempfile.TemporaryDirectory(prefix="gate-lock-test-") as temporary:
            root = Path(temporary)
            scripts = root / "scripts"
            scripts.mkdir()
            shutil.copy2(ROOT / "scripts" / "gate_runner.py", scripts / "gate_runner.py")
            (scripts / "slow.py").write_text(
                "from pathlib import Path\n"
                "import os\n"
                "import time\n"
                "runs = Path('runs')\n"
                "runs.mkdir(exist_ok=True)\n"
                "(runs / str(os.getpid())).write_text('ran', encoding='utf-8')\n"
                "time.sleep(0.5)\n",
                encoding="utf-8",
            )
            synthetic_manifest = """[inventory]
schema = 1
tiers = ["canonical", "doctor", "pr", "full", "nightly", "stop", "legacy", "synthetic"]

[[gate]]
id = "test.once"
order = 1
description = "cross-process execution-lock canary"
tiers = ["synthetic"]
phase = "provenance"
command = ["{python}", "scripts/slow.py"]
self_test = ["{python}", "scripts/slow.py"]
timeout_seconds = 10
cache = "content-hash"
inputs = ["scripts/slow.py"]
path_triggers = ["scripts/slow.py"]
"""
            manifest = scripts / "gates.toml"
            manifest.write_text(
                synthetic_manifest + "\n" + mandatory_gate_blocks(),
                encoding="utf-8",
            )
            environment = os.environ.copy()
            environment["CIVSIM_GATE_CACHE_DIR"] = str(root / "cache")
            command = [
                sys.executable,
                str(scripts / "gate_runner.py"),
                "--manifest",
                str(manifest),
                "run",
                "--id",
                "test.once",
                "--jobs",
                "1",
            ]
            workers = [
                subprocess.Popen(
                    command,
                    cwd=root,
                    env=environment,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                for _ in range(2)
            ]
            results = [worker.communicate(timeout=20) for worker in workers]
            for worker, (stdout, stderr) in zip(workers, results, strict=True):
                self.assertEqual(worker.returncode, 0, stdout + stderr)
            output = "\n".join(stdout for stdout, _ in results)
            self.assertEqual(len(list((root / "runs").iterdir())), 1)
            self.assertEqual(output.count("[PASS] test.once"), 1)
            self.assertEqual(output.count("[CACHED] test.once"), 1)

            shutil.rmtree(root / "runs")
            (scripts / "slow.py").write_text(
                "from pathlib import Path\n"
                "import os\n"
                "import time\n"
                "runs = Path('runs')\n"
                "active = Path('active')\n"
                "runs.mkdir(exist_ok=True)\n"
                "active.mkdir(exist_ok=True)\n"
                "pid = str(os.getpid())\n"
                "(runs / pid).write_text('ran', encoding='utf-8')\n"
                "marker = active / pid\n"
                "marker.write_text('active', encoding='utf-8')\n"
                "if len(list(active.iterdir())) > 1:\n"
                "    Path('overlap').write_text('detected', encoding='utf-8')\n"
                "time.sleep(0.5)\n"
                "marker.unlink()\n",
                encoding="utf-8",
            )
            manifest.write_text(
                synthetic_manifest.replace(
                    'cache = "content-hash"',
                    'cache = "never"\nno_cache_reason = "serialization canary"',
                    1,
                )
                + "\n"
                + mandatory_gate_blocks(),
                encoding="utf-8",
            )
            workers = [
                subprocess.Popen(
                    command,
                    cwd=root,
                    env=environment,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                for _ in range(2)
            ]
            results = [worker.communicate(timeout=20) for worker in workers]
            for worker, (stdout, stderr) in zip(workers, results, strict=True):
                self.assertEqual(worker.returncode, 0, stdout + stderr)
            output = "\n".join(stdout for stdout, _ in results)
            self.assertEqual(len(list((root / "runs").iterdir())), 2)
            self.assertFalse((root / "overlap").exists())
            self.assertEqual(output.count("[PASS] test.once"), 2)
            self.assertNotIn("[CACHED] test.once", output)

    def test_parallel_workers_overlap_with_phase_barriers_and_authority_order(self) -> None:
        with tempfile.TemporaryDirectory(prefix="gate-parallel-test-") as temporary:
            root = Path(temporary)
            scripts = root / "scripts"
            scripts.mkdir()
            shutil.copy2(ROOT / "scripts" / "gate_runner.py", scripts / "gate_runner.py")
            (scripts / "pre.py").write_text(
                "from pathlib import Path\nimport time\n"
                "time.sleep(0.3)\nPath('pre.done').write_text('ok')\nprint('pre')\n",
                encoding="utf-8",
            )
            for name, other in (("first", "second"), ("second", "first")):
                (scripts / f"{name}.py").write_text(
                    "from pathlib import Path\nimport time\n"
                    "assert Path('pre.done').exists()\n"
                    f"Path('{name}.ready').write_text('ok')\n"
                    "deadline = time.monotonic() + 3\n"
                    f"while not Path('{other}.ready').exists():\n"
                    "    if time.monotonic() > deadline:\n"
                    "        raise SystemExit('parallel rendezvous timed out')\n"
                    "    time.sleep(0.01)\n"
                    f"print({name!r})\n",
                    encoding="utf-8",
                )
            (scripts / "post.py").write_text(
                "from pathlib import Path\n"
                "assert Path('first.ready').exists()\n"
                "assert Path('second.ready').exists()\n"
                "print('post')\n",
                encoding="utf-8",
            )
            synthetic_manifest = """[inventory]
schema = 1
tiers = ["canonical", "doctor", "pr", "full", "nightly", "stop", "synthetic"]

[[gate]]
id = "test.pre"
order = 5
description = "pre"
tiers = ["synthetic"]
phase = "pre"
command = ["{python}", "scripts/pre.py"]
self_test = ["{python}", "scripts/pre.py"]
timeout_seconds = 10
cache = "never"
no_cache_reason = "phase canary"
inputs = ["scripts/pre.py"]
path_triggers = ["scripts/pre.py"]

[[gate]]
id = "test.first"
order = 10
description = "first"
tiers = ["synthetic"]
phase = "provenance"
command = ["{python}", "scripts/first.py"]
self_test = ["{python}", "scripts/first.py"]
timeout_seconds = 10
cache = "never"
no_cache_reason = "parallel canary"
inputs = ["scripts/first.py"]
path_triggers = ["scripts/first.py"]

[[gate]]
id = "test.second"
order = 20
description = "second"
tiers = ["synthetic"]
phase = "provenance"
command = ["{python}", "scripts/second.py"]
self_test = ["{python}", "scripts/second.py"]
timeout_seconds = 10
cache = "never"
no_cache_reason = "parallel canary"
inputs = ["scripts/second.py"]
path_triggers = ["scripts/second.py"]

[[gate]]
id = "test.post"
order = 30
description = "post"
tiers = ["synthetic"]
phase = "post"
command = ["{python}", "scripts/post.py"]
self_test = ["{python}", "scripts/post.py"]
timeout_seconds = 10
cache = "never"
no_cache_reason = "phase canary"
inputs = ["scripts/post.py"]
path_triggers = ["scripts/post.py"]
"""
            (scripts / "gates.toml").write_text(
                synthetic_manifest + "\n" + mandatory_gate_blocks(),
                encoding="utf-8",
            )
            result = subprocess.run(
                [
                    sys.executable,
                    str(scripts / "gate_runner.py"),
                    "run",
                    "--tier",
                    "synthetic",
                    "--jobs",
                    "2",
                    "--no-cache",
                ],
                cwd=root,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertLess(result.stdout.index("pre"), result.stdout.index("first"))
            self.assertLess(result.stdout.index("first"), result.stdout.index("second"))
            self.assertLess(result.stdout.index("second"), result.stdout.index("post"))

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

    def test_stone0_history_scan_works_without_process_substitution(self) -> None:
        bash = self.require_bash()
        with tempfile.TemporaryDirectory(prefix="stone0-history-test-") as temporary:
            root = Path(temporary) / "repo"
            root.mkdir()
            scripts = root / "scripts"
            scripts.mkdir()
            shutil.copy2(
                ROOT / "scripts" / "stone0-pre-push-hook.sh",
                scripts / "stone0-pre-push-hook.sh",
            )
            (scripts / "stone0-pre-push-hook.sh").chmod(0o755)
            (scripts / "stone0_tombstones.txt").write_text("", encoding="utf-8")
            secret = Path(temporary) / "synthetic-secret.pass"
            token = "stone0-synthetic-live-token-4d8f"
            secret.write_text(token + "\n", encoding="utf-8")

            subprocess.run(["git", "init", "-q"], cwd=root, check=True)
            subprocess.run(
                ["git", "config", "user.email", "stone0@example.invalid"],
                cwd=root,
                check=True,
            )
            subprocess.run(
                ["git", "config", "user.name", "Stone0 Test"],
                cwd=root,
                check=True,
            )
            (root / "clean.txt").write_text("clean\n", encoding="utf-8")
            subprocess.run(["git", "add", "-A"], cwd=root, check=True)
            subprocess.run(
                ["git", "commit", "-q", "-m", "clean"], cwd=root, check=True
            )
            clean_sha = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()

            environment = os.environ.copy()
            environment["STONE0_SECRETS_PATH"] = secret.as_posix()
            zero = "0" * 40
            clean_update = f"refs/heads/topic {clean_sha} refs/heads/topic {zero}\n"
            clean = subprocess.run(
                [bash, "scripts/stone0-pre-push-hook.sh", "origin", "ssh://example/repo"],
                cwd=root,
                input=clean_update,
                capture_output=True,
                text=True,
                check=False,
                env=environment,
            )
            self.assertEqual(clean.returncode, 0, clean.stdout + clean.stderr)
            self.assertNotIn("Could not scan commit", clean.stderr)

            (root / "laundered.txt").write_text(token + "\n", encoding="utf-8")
            subprocess.run(["git", "add", "-A"], cwd=root, check=True)
            subprocess.run(
                ["git", "commit", "-q", "-m", "laundered"], cwd=root, check=True
            )
            bad_sha = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=root,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            bad_update = (
                f"refs/heads/topic {bad_sha} refs/heads/topic {clean_sha}\n"
            )
            blocked = subprocess.run(
                [bash, "scripts/stone0-pre-push-hook.sh", "origin", "ssh://example/repo"],
                cwd=root,
                input=bad_update,
                capture_output=True,
                text=True,
                check=False,
                env=environment,
            )
            self.assertEqual(blocked.returncode, 1, blocked.stdout + blocked.stderr)
            self.assertIn("stone0 pre-push: BLOCKED", blocked.stdout)
            self.assertNotIn(token, blocked.stdout + blocked.stderr)

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
        self.assertIn(gate_runner.POLICY_DETECTION_MARKER, stone0)

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
        for task in ("check-fast", "cache-info", "trim-wsl"):
            self.assertIn(f'"{task}"', powershell)
        self.assertIn("source scripts/wsl_dev_env.sh --quiet", powershell)

        justfile = (ROOT / "justfile").read_text(encoding="utf-8")
        self.assertIn(
            "python3 scripts/gate_runner.py list --tier {{tier}} --ids-only",
            justfile,
        )
        self.assertIn('env_var_or_default("CIVSIM_PARKED_TARGET_DIR"', justfile)
        self.assertIn('cargo_dev := "bash scripts/cargo_dev.sh"', justfile)
        self.assertNotRegex(justfile, r"(?m)^\s+cargo (?:run|test|check|clippy|doc|build)")
        for recipe in ("check-fast:", "cache-info:", "trim-wsl:"):
            self.assertIn(recipe, justfile)

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
