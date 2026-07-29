#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

"""Compile canonical Cargo tests once, then run complete test binaries in parallel.

Cargo runs test executables serially. This runner preserves one process per
libtest binary, including each binary's OnceLock fixtures, while scheduling
independent binaries concurrently. Cargo metadata independently cross-checks
the executable inventory before any success can be reported.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import datetime as dt
import functools
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from typing import Any, Iterable, Mapping, Sequence


ROOT = Path(__file__).resolve().parent.parent
PACKAGE_RE = re.compile(r"^[A-Za-z0-9_-]+$")
TEST_LINE_RE = re.compile(r": (?:test|benchmark)$")
GPU_PACKAGE = "civsim-gpu"
GPU_LOCK = threading.Lock()
RUN_MARKER = ".civsim-parallel-test-run-v1"
MAX_RETAINED_RUNS = 32
MAX_RUN_AGE_SECONDS = 30 * 24 * 60 * 60
MIN_COUNT_PRUNE_AGE_SECONDS = 60 * 60


class SchedulerError(RuntimeError):
    """The complete Cargo test inventory could not be proved or executed."""


@dataclasses.dataclass(frozen=True, order=True)
class TargetKey:
    package_id: str
    name: str
    kinds: tuple[str, ...]

    def label(self, package_name: str) -> str:
        return f"{package_name}:{'+'.join(self.kinds)}:{self.name}"


@dataclasses.dataclass(frozen=True)
class TestTarget:
    key: TargetKey
    package_name: str
    package: Mapping[str, Any]
    executable: Path
    executable_sha256: str
    test_count: int

    @property
    def label(self) -> str:
        return self.key.label(self.package_name)

    @property
    def uses_gpu(self) -> bool:
        return self.package_name == GPU_PACKAGE


@dataclasses.dataclass(frozen=True)
class TargetResult:
    label: str
    returncode: int
    wall_seconds: float
    log_path: Path
    executable_sha256: str
    test_count: int


def _positive_integer(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a positive integer") from error
    if value < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return value


def _default_jobs() -> int:
    raw = os.environ.get("CIVSIM_TEST_JOBS")
    if raw:
        try:
            return _positive_integer(raw)
        except argparse.ArgumentTypeError:
            return 1
    available = (
        len(os.sched_getaffinity(0))
        if hasattr(os, "sched_getaffinity")
        else (os.cpu_count() or 1)
    )
    return max(1, min(8, available))


def _cargo_executable() -> Path:
    declared = os.environ.get("CARGO")
    candidates: list[str] = []
    if declared:
        candidates.append(declared)
    try:
        selected = subprocess.run(
            ["rustup", "which", "cargo"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=10,
            check=True,
        ).stdout.strip()
        if selected:
            candidates.append(selected)
    except (OSError, subprocess.SubprocessError, UnicodeError):
        pass
    found = shutil.which("cargo")
    if found:
        candidates.append(found)
    for candidate in candidates:
        try:
            path = Path(candidate).expanduser().resolve(strict=True)
        except OSError:
            continue
        if path.is_file() and os.access(path, os.X_OK):
            return path
    raise SchedulerError("the launching Cargo executable is unavailable")


def _cargo_metadata(cargo: Path) -> Mapping[str, Any]:
    command = [
        str(cargo),
        "metadata",
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
    ]
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=60,
            check=True,
        )
        return json.loads(result.stdout)
    except (
        OSError,
        subprocess.SubprocessError,
        UnicodeError,
        json.JSONDecodeError,
    ) as error:
        raise SchedulerError(f"Cargo metadata failed: {error}") from error


def _compile_test_artifacts(packages: Sequence[str]) -> list[Mapping[str, Any]]:
    command = ["bash", "scripts/cargo_dev.sh", "test"]
    for package in packages:
        command.extend(("-p", package))
    command.extend(
        (
            "--all-targets",
            "--no-run",
            "--message-format=json-render-diagnostics",
        )
    )
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=None,
            text=True,
            encoding="utf-8",
            errors="strict",
            check=False,
        )
    except (OSError, UnicodeError) as error:
        raise SchedulerError(f"Cargo test compilation could not start: {error}") from error
    if result.returncode != 0:
        raise SchedulerError(
            f"Cargo test compilation failed with exit {result.returncode}"
        )
    records: list[Mapping[str, Any]] = []
    for line_number, line in enumerate(result.stdout.splitlines(), start=1):
        try:
            decoded = json.loads(line)
        except json.JSONDecodeError as error:
            raise SchedulerError(
                f"Cargo emitted malformed JSON on line {line_number}: {error}"
            ) from error
        if isinstance(decoded, dict):
            records.append(decoded)
    return records


def _package_map(
    metadata: Mapping[str, Any], requested: Sequence[str]
) -> dict[str, Mapping[str, Any]]:
    by_name = {
        str(package["name"]): package
        for package in metadata.get("packages", [])
        if isinstance(package, dict) and "name" in package
    }
    missing = sorted(set(requested) - set(by_name))
    if missing:
        raise SchedulerError(
            "Cargo metadata omitted requested package(s): " + ", ".join(missing)
        )
    return {str(by_name[name]["id"]): by_name[name] for name in requested}


def _target_key(package_id: str, target: Mapping[str, Any]) -> TargetKey:
    kinds = target.get("kind")
    if not isinstance(kinds, list) or not kinds or not all(
        isinstance(kind, str) for kind in kinds
    ):
        raise SchedulerError("Cargo target kind is absent or malformed")
    name = target.get("name")
    if not isinstance(name, str) or not name:
        raise SchedulerError("Cargo target name is absent or malformed")
    return TargetKey(package_id, name, tuple(kinds))


def _expected_targets(
    packages: Mapping[str, Mapping[str, Any]]
) -> set[TargetKey]:
    expected: set[TargetKey] = set()
    for package_id, package in packages.items():
        targets = package.get("targets")
        if not isinstance(targets, list):
            raise SchedulerError(f"package {package_id} has no target inventory")
        for target in targets:
            if not isinstance(target, dict):
                continue
            key = _target_key(package_id, target)
            if "custom-build" in key.kinds:
                continue
            if key in expected:
                raise SchedulerError(f"duplicate metadata test target {key}")
            expected.add(key)
    return expected


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise SchedulerError(f"test executable cannot be read: {path}") from error
    return digest.hexdigest()


def _checked_executable(raw: str, target_root: Path) -> Path:
    candidate = Path(raw)
    if candidate.is_symlink():
        raise SchedulerError(f"test executable is a symbolic link: {candidate}")
    try:
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(target_root.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise SchedulerError(
            f"test executable is outside the managed target: {candidate}"
        ) from error
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise SchedulerError(f"test executable is not runnable: {resolved}")
    return resolved


def _discovered_targets(
    records: Iterable[Mapping[str, Any]],
    packages: Mapping[str, Mapping[str, Any]],
    target_root: Path,
) -> dict[TargetKey, tuple[Path, str]]:
    discovered: dict[TargetKey, tuple[Path, str]] = {}
    for record in records:
        if record.get("reason") != "compiler-artifact":
            continue
        package_id = str(record.get("package_id", ""))
        if package_id not in packages:
            continue
        profile = record.get("profile")
        executable = record.get("executable")
        target = record.get("target")
        if not isinstance(profile, dict) or profile.get("test") is not True:
            continue
        if not isinstance(executable, str) or not executable:
            raise SchedulerError("a selected test artifact has no executable")
        if not isinstance(target, dict):
            raise SchedulerError("a selected test artifact has no target record")
        key = _target_key(package_id, target)
        checked = _checked_executable(executable, target_root)
        content_sha256 = _sha256_file(checked)
        previous = discovered.get(key)
        current = (checked, content_sha256)
        if previous is not None and previous != current:
            raise SchedulerError(f"test target {key} maps to multiple executables")
        discovered[key] = current
    return discovered


def _version_parts(raw: str) -> tuple[str, str, str, str]:
    core, separator, prerelease = raw.partition("-")
    parts = core.split(".")
    if len(parts) != 3:
        raise SchedulerError(f"package version is malformed: {raw}")
    return parts[0], parts[1], parts[2], prerelease if separator else ""


@functools.lru_cache(maxsize=4)
def _rust_runtime_paths(cargo: Path, target_root: Path) -> tuple[str, ...]:
    try:
        rustc = cargo.parent / "rustc"
        verbose = subprocess.run(
            [str(rustc), "-vV"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=10,
            check=True,
        ).stdout
        host = next(
            line.partition(":")[2].strip()
            for line in verbose.splitlines()
            if line.startswith("host:")
        )
        sysroot = Path(
            subprocess.run(
                [str(rustc), "--print", "sysroot"],
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=10,
                check=True,
            ).stdout.strip()
        ).resolve(strict=True)
    except (
        OSError,
        subprocess.SubprocessError,
        UnicodeError,
        StopIteration,
    ) as error:
        raise SchedulerError(f"Rust runtime library paths are unavailable: {error}") from error
    runtime_paths = [
        target_root / "debug",
        target_root / "debug" / "deps",
        sysroot / "lib" / "rustlib" / host / "lib",
        sysroot / "lib",
    ]
    return tuple(str(path) for path in runtime_paths)


def _target_environment(
    package: Mapping[str, Any], cargo: Path, target_root: Path
) -> dict[str, str]:
    environment = os.environ.copy()
    manifest = Path(str(package["manifest_path"])).resolve(strict=True)
    major, minor, patch, prerelease = _version_parts(str(package["version"]))
    package_values = {
        "CARGO": str(cargo),
        "CARGO_MANIFEST_DIR": str(manifest.parent),
        "CARGO_MANIFEST_PATH": str(manifest),
        "CARGO_PKG_AUTHORS": ":".join(str(value) for value in package.get("authors", [])),
        "CARGO_PKG_DESCRIPTION": str(package.get("description") or ""),
        "CARGO_PKG_HOMEPAGE": str(package.get("homepage") or ""),
        "CARGO_PKG_LICENSE": str(package.get("license") or ""),
        "CARGO_PKG_LICENSE_FILE": str(package.get("license_file") or ""),
        "CARGO_PKG_NAME": str(package["name"]),
        "CARGO_PKG_README": str(package.get("readme") or ""),
        "CARGO_PKG_REPOSITORY": str(package.get("repository") or ""),
        "CARGO_PKG_RUST_VERSION": str(package.get("rust_version") or ""),
        "CARGO_PKG_VERSION": str(package["version"]),
        "CARGO_PKG_VERSION_MAJOR": major,
        "CARGO_PKG_VERSION_MINOR": minor,
        "CARGO_PKG_VERSION_PATCH": patch,
        "CARGO_PKG_VERSION_PRE": prerelease,
    }
    environment.update(package_values)

    runtime_paths = [Path(path) for path in _rust_runtime_paths(cargo, target_root)]
    inherited = environment.get("LD_LIBRARY_PATH", "")
    if inherited:
        runtime_paths.extend(Path(value) for value in inherited.split(os.pathsep) if value)
    environment["LD_LIBRARY_PATH"] = os.pathsep.join(
        dict.fromkeys(str(path) for path in runtime_paths)
    )
    return environment


def _list_test_count(
    key: TargetKey,
    package: Mapping[str, Any],
    executable: Path,
    cargo: Path,
    target_root: Path,
) -> int:
    environment = _target_environment(package, cargo, target_root)
    try:
        result = subprocess.run(
            [str(executable), "--list"],
            cwd=Path(str(package["manifest_path"])).parent,
            env=environment,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=60,
            check=False,
        )
    except (OSError, subprocess.SubprocessError, UnicodeError) as error:
        raise SchedulerError(f"test listing failed for {key}: {error}") from error
    if result.returncode != 0:
        raise SchedulerError(
            f"test listing failed for {key} with exit {result.returncode}: "
            + result.stderr.strip()
        )
    return sum(1 for line in result.stdout.splitlines() if TEST_LINE_RE.search(line))


def _prepare_targets(
    discovered: Mapping[TargetKey, tuple[Path, str]],
    packages: Mapping[str, Mapping[str, Any]],
    cargo: Path,
    target_root: Path,
    jobs: int,
) -> tuple[TestTarget, ...]:
    def prepare(item: tuple[TargetKey, tuple[Path, str]]) -> TestTarget:
        key, (executable, content_sha256) = item
        package = packages[key.package_id]
        return TestTarget(
            key=key,
            package_name=str(package["name"]),
            package=package,
            executable=executable,
            executable_sha256=content_sha256,
            test_count=_list_test_count(
                key, package, executable, cargo, target_root
            ),
        )

    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as executor:
        targets = tuple(executor.map(prepare, discovered.items()))
    return targets


def _priority(target: TestTarget) -> tuple[int, int, str]:
    kinds = set(target.key.kinds)
    if "test" in kinds:
        kind_priority = 0
    elif "lib" in kinds:
        kind_priority = 1
    elif "bin" in kinds:
        kind_priority = 2
    elif "example" in kinds:
        kind_priority = 3
    else:
        kind_priority = 4
    if target.uses_gpu:
        kind_priority += 10
    return kind_priority, -target.test_count, target.label


def _safe_log_name(target: TestTarget) -> str:
    digest = hashlib.sha256(target.label.encode("utf-8")).hexdigest()[:12]
    readable = re.sub(r"[^A-Za-z0-9_.-]+", "_", target.label)[:100]
    return f"{readable}-{digest}.log"


def _run_target(
    target: TestTarget,
    cargo: Path,
    target_root: Path,
    log_dir: Path,
) -> TargetResult:
    environment = _target_environment(target.package, cargo, target_root)
    log_path = log_dir / _safe_log_name(target)
    started = time.monotonic()
    lock = GPU_LOCK if target.uses_gpu else _NullLock()
    with lock:
        with log_path.open("wb") as log:
            try:
                result = subprocess.run(
                    [str(target.executable)],
                    cwd=Path(str(target.package["manifest_path"])).parent,
                    env=environment,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    check=False,
                )
                returncode = result.returncode
            except OSError as error:
                log.write(f"scheduler launch failure: {error}\n".encode("utf-8"))
                returncode = 127
    observed_sha256 = _sha256_file(target.executable)
    if observed_sha256 != target.executable_sha256:
        with log_path.open("ab") as log:
            log.write(b"test executable changed during execution\n")
        returncode = 2
    return TargetResult(
        label=target.label,
        returncode=returncode,
        wall_seconds=time.monotonic() - started,
        log_path=log_path,
        executable_sha256=target.executable_sha256,
        test_count=target.test_count,
    )


class _NullLock:
    def __enter__(self) -> None:
        return None

    def __exit__(self, *_arguments: object) -> None:
        return None


def _run_parallel(
    targets: Sequence[TestTarget],
    cargo: Path,
    target_root: Path,
    log_dir: Path,
    jobs: int,
) -> tuple[TargetResult, ...]:
    ordered = sorted(targets, key=_priority)
    results: list[TargetResult] = []
    running: dict[concurrent.futures.Future[TargetResult], TestTarget] = {}
    pending = iter(ordered)
    stop_launching = False
    def fill(executor: concurrent.futures.ThreadPoolExecutor) -> None:
        if stop_launching:
            return
        while len(running) < jobs:
            try:
                target = next(pending)
            except StopIteration:
                return
            print(
                f"[TEST START] {target.label} ({target.test_count} test(s))",
                flush=True,
            )
            future = executor.submit(
                _run_target, target, cargo, target_root, log_dir
            )
            running[future] = target

    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as executor:
        fill(executor)
        while running:
            done, _ = concurrent.futures.wait(
                running,
                timeout=15,
                return_when=concurrent.futures.FIRST_COMPLETED,
            )
            if not done:
                labels = ", ".join(sorted(target.label for target in running.values()))
                print(f"[TEST ACTIVE] {labels}", flush=True)
                continue
            for future in done:
                target = running.pop(future)
                try:
                    result = future.result()
                except Exception as error:
                    result = TargetResult(
                        label=target.label,
                        returncode=2,
                        wall_seconds=0.0,
                        log_path=log_dir / _safe_log_name(target),
                        executable_sha256=target.executable_sha256,
                        test_count=target.test_count,
                    )
                    result.log_path.write_text(
                        f"scheduler worker failure: {error}\n", encoding="utf-8"
                    )
                results.append(result)
                status = "PASS" if result.returncode == 0 else f"FAIL({result.returncode})"
                print(
                    f"[TEST {status}] {result.label} in {result.wall_seconds:.2f}s",
                    flush=True,
                )
                if result.returncode != 0:
                    stop_launching = True
                    try:
                        sys.stdout.flush()
                        sys.stdout.buffer.write(result.log_path.read_bytes())
                        sys.stdout.buffer.flush()
                    except OSError:
                        pass
            fill(executor)

    if not stop_launching:
        try:
            next(pending)
        except StopIteration:
            pass
        else:
            raise SchedulerError("the scheduler stopped before the inventory was exhausted")
    return tuple(results)


def _default_receipt_root() -> Path:
    cache = os.environ.get("CIVSIM_REPO_CACHE_DIR")
    base = Path(cache) if cache else ROOT / ".git" / "civsim-cache"
    return base / "test-runs"


def _default_receipt_dir() -> Path:
    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    return _default_receipt_root() / f"{timestamp}-{os.getpid()}"


def _prepare_receipt_directory(path: Path) -> Path:
    if path.exists() or path.is_symlink():
        raise SchedulerError(f"test receipt directory already exists: {path}")
    try:
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        path.mkdir(mode=0o700)
        (path / RUN_MARKER).write_text(RUN_MARKER + "\n", encoding="ascii")
        log_dir = path / "logs"
        log_dir.mkdir(mode=0o700)
    except OSError as error:
        raise SchedulerError(f"test receipt directory is unavailable: {error}") from error
    return log_dir


def _prune_owned_runs(root: Path, current: Path) -> None:
    try:
        resolved_root = root.resolve(strict=True)
        resolved_current = current.resolve(strict=True)
        resolved_current.relative_to(resolved_root)
    except (OSError, ValueError) as error:
        raise SchedulerError(f"test receipt retention root is unsafe: {error}") from error
    owned: list[tuple[int, Path]] = []
    try:
        for entry in resolved_root.iterdir():
            if entry.is_symlink() or not entry.is_dir():
                continue
            resolved = entry.resolve(strict=True)
            resolved.relative_to(resolved_root)
            marker = resolved / RUN_MARKER
            if (
                marker.is_symlink()
                or not marker.is_file()
                or marker.read_text(encoding="ascii") != RUN_MARKER + "\n"
            ):
                continue
            owned.append((resolved.stat().st_mtime_ns, resolved))
    except (OSError, UnicodeError, ValueError) as error:
        raise SchedulerError(f"test receipt retention scan failed: {error}") from error
    owned.sort(reverse=True)
    now = time.time()
    for index, (_, path) in enumerate(owned):
        if path == resolved_current:
            continue
        try:
            age = max(0.0, now - path.stat().st_mtime)
        except OSError as error:
            raise SchedulerError(
                f"test receipt retention metadata failed: {error}"
            ) from error
        over_count = (
            index >= MAX_RETAINED_RUNS
            and age >= MIN_COUNT_PRUNE_AGE_SECONDS
        )
        if age < MAX_RUN_AGE_SECONDS and not over_count:
            continue
        try:
            path.relative_to(resolved_root)
            shutil.rmtree(path)
        except (OSError, ValueError) as error:
            raise SchedulerError(f"could not prune owned test receipt {path}: {error}") from error


def _atomic_json(path: Path, payload: Mapping[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def _self_test() -> int:
    package_id = "path+file:///synthetic#fixture@0.1.0"
    package = {
        "id": package_id,
        "name": "fixture",
        "version": "0.1.0",
        "manifest_path": str(ROOT / "Cargo.toml"),
        "targets": [
            {
                "name": "fixture",
                "kind": ["lib"],
                "test": True,
            }
        ],
    }
    packages = {package_id: package}
    expected = _expected_targets(packages)
    if expected != {TargetKey(package_id, "fixture", ("lib",))}:
        raise AssertionError("metadata target cross-check lost the synthetic test")
    with tempfile.TemporaryDirectory(prefix="civsim-test-scheduler-") as raw:
        outside = Path(raw).resolve()
        target_root = outside / "target"
        target_root.mkdir()
        executable = target_root / "fixture-test"
        executable.write_text(
            "#!/usr/bin/env python3\n"
            "import os\n"
            "import sys\n"
            "if '--list' in sys.argv:\n"
            "    print('fixture::passes: test')\n"
            "elif os.environ.get('CARGO_PKG_NAME') != 'fixture':\n"
            "    raise SystemExit(4)\n",
            encoding="utf-8",
        )
        executable.chmod(0o755)
        record = {
            "reason": "compiler-artifact",
            "package_id": package_id,
            "profile": {"test": True},
            "target": {"name": "fixture", "kind": ["lib"]},
            "executable": str(executable),
        }
        discovered = _discovered_targets([record], packages, target_root)
        if set(discovered) != expected:
            raise AssertionError("synthetic compiled inventory did not match metadata")

        cargo = _cargo_executable()
        key = next(iter(expected))
        content_sha256 = discovered[key][1]
        if (
            _list_test_count(key, package, executable, cargo, target_root)
            != 1
        ):
            raise AssertionError("synthetic libtest listing was not counted")
        target = TestTarget(
            key=key,
            package_name="fixture",
            package=package,
            executable=executable,
            executable_sha256=content_sha256,
            test_count=1,
        )
        log_dir = outside / "logs"
        log_dir.mkdir()
        if _run_target(target, cargo, target_root, log_dir).returncode != 0:
            raise AssertionError("synthetic test target did not run")

        linked = target_root / "linked-test"
        linked.symlink_to(executable)
        try:
            _checked_executable(str(linked), target_root)
        except SchedulerError:
            pass
        else:
            raise AssertionError("a linked test executable was accepted")

        try:
            _discovered_targets(
                [{**record, "executable": str(ROOT / "Cargo.toml")}],
                packages,
                target_root,
            )
        except SchedulerError:
            pass
        else:
            raise AssertionError("an executable outside the managed target was accepted")

        rendezvous_targets: list[TestTarget] = []
        for name, other in (("first", "second"), ("second", "first")):
            path = target_root / f"{name}-test"
            own_marker = outside / f"{name}.ready"
            other_marker = outside / f"{other}.ready"
            path.write_text(
                "#!/usr/bin/env python3\n"
                "from pathlib import Path\n"
                "import time\n"
                f"Path({str(own_marker)!r}).write_text('ready', encoding='utf-8')\n"
                "deadline = time.monotonic() + 2\n"
                f"while not Path({str(other_marker)!r}).exists():\n"
                "    if time.monotonic() >= deadline:\n"
                "        raise SystemExit(5)\n"
                "    time.sleep(0.01)\n",
                encoding="utf-8",
            )
            path.chmod(0o755)
            rendezvous_targets.append(
                TestTarget(
                    key=TargetKey(package_id, name, ("test",)),
                    package_name="fixture",
                    package=package,
                    executable=path,
                    executable_sha256=_sha256_file(path),
                    test_count=1,
                )
            )
        parallel_results = _run_parallel(
            rendezvous_targets,
            cargo,
            target_root,
            log_dir,
            2,
        )
        if len(parallel_results) != 2 or any(
            result.returncode != 0 for result in parallel_results
        ):
            raise AssertionError("synthetic complete binaries did not overlap")

        mutating = target_root / "mutating-test"
        mutating.write_text(
            "#!/usr/bin/env python3\n"
            "from pathlib import Path\n"
            "path = Path(__file__)\n"
            "path.write_text('#!/usr/bin/env python3\\n', encoding='utf-8')\n",
            encoding="utf-8",
        )
        mutating.chmod(0o755)
        mutation_target = TestTarget(
            key=TargetKey(package_id, "mutating", ("test",)),
            package_name="fixture",
            package=package,
            executable=mutating,
            executable_sha256=_sha256_file(mutating),
            test_count=1,
        )
        if _run_target(
            mutation_target, cargo, target_root, log_dir
        ).returncode != 2:
            raise AssertionError("test executable mutation was not refused")

        payload = {
            "schema": "civsim.parallel-cargo-test.self-test.v1",
            "expected": [dataclasses.asdict(key) for key in sorted(expected)],
        }
        receipt = outside / "receipt.json"
        _atomic_json(receipt, payload)
        if json.loads(receipt.read_text(encoding="utf-8"))["schema"] != payload["schema"]:
            raise AssertionError("the scheduler receipt did not round trip")

        retention_root = outside / "retention"
        current = retention_root / "current"
        _prepare_receipt_directory(current)
        old_time = time.time() - (2 * MIN_COUNT_PRUNE_AGE_SECONDS)
        for index in range(MAX_RETAINED_RUNS + 1):
            run = retention_root / f"old-{index:02d}"
            run.mkdir()
            (run / RUN_MARKER).write_text(RUN_MARKER + "\n", encoding="ascii")
            os.utime(run, (old_time, old_time))
        unowned = retention_root / "unowned"
        unowned.mkdir()
        os.utime(unowned, (old_time, old_time))
        linked_run = retention_root / "linked"
        linked_run.symlink_to(unowned, target_is_directory=True)
        _prune_owned_runs(retention_root, current)
        owned_count = sum(
            1
            for path in retention_root.iterdir()
            if path.is_dir()
            and not path.is_symlink()
            and (path / RUN_MARKER).is_file()
        )
        if owned_count != MAX_RETAINED_RUNS:
            raise AssertionError("test receipt retention did not enforce its bound")
        if not unowned.exists() or not linked_run.is_symlink():
            raise AssertionError("test receipt retention touched unowned state")
    print("parallel Cargo test scheduler self-test: PASS")
    return 0


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("-p", "--package", action="append", default=[])
    parser.add_argument("--jobs", type=_positive_integer, default=_default_jobs())
    parser.add_argument("--receipt-dir", type=Path)
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    if args.self_test:
        return _self_test()
    if not args.package:
        raise SystemExit("at least one -p/--package is required")
    if len(set(args.package)) != len(args.package):
        raise SystemExit("duplicate package selection")
    invalid = [package for package in args.package if not PACKAGE_RE.fullmatch(package)]
    if invalid:
        raise SystemExit("invalid package name(s): " + ", ".join(invalid))

    target_raw = os.environ.get("CARGO_TARGET_DIR")
    if not target_raw:
        raise SystemExit(
            "CARGO_TARGET_DIR is absent; source scripts/wsl_dev_env.sh before scheduling"
        )
    target_root = Path(target_raw).expanduser().resolve(strict=True)
    cargo = _cargo_executable()
    managed_receipt_root = (
        _default_receipt_root().expanduser().resolve()
        if args.receipt_dir is None
        else None
    )
    receipt_dir = (args.receipt_dir or _default_receipt_dir()).expanduser().resolve()

    started = time.monotonic()
    metadata = _cargo_metadata(cargo)
    packages = _package_map(metadata, args.package)
    expected = _expected_targets(packages)
    artifacts = _compile_test_artifacts(args.package)
    discovered = _discovered_targets(artifacts, packages, target_root)
    discovered_keys = set(discovered)
    if expected != discovered_keys:
        missing = sorted(expected - discovered_keys)
        extra = sorted(discovered_keys - expected)
        raise SchedulerError(
            "Cargo metadata and compiled test inventory disagree: "
            f"missing={missing}, extra={extra}"
        )
    targets = _prepare_targets(
        discovered, packages, cargo, target_root, args.jobs
    )
    if len(targets) != len(expected):
        raise SchedulerError("prepared test inventory changed cardinality")
    if args.list:
        for target in sorted(targets, key=_priority):
            print(f"{target.test_count:4}  {target.label}  {target.executable}")
        return 0

    log_dir = _prepare_receipt_directory(receipt_dir)
    print(
        f"parallel Cargo tests: {len(targets)} complete binary target(s), "
        f"{sum(target.test_count for target in targets)} listed test(s), "
        f"{args.jobs} worker(s)",
        flush=True,
    )
    results = _run_parallel(
        targets, cargo, target_root, log_dir, args.jobs
    )
    result_by_label = {result.label: result for result in results}
    expected_labels = {target.label for target in targets}
    if set(result_by_label) != expected_labels:
        missing = sorted(expected_labels - set(result_by_label))
        raise SchedulerError(f"test execution inventory is incomplete: {missing}")
    failed = [result for result in results if result.returncode != 0]

    receipt = {
        "schema": "civsim.parallel-cargo-test.v1",
        "authority_effect": "none",
        "packages": list(args.package),
        "jobs": args.jobs,
        "target_count": len(targets),
        "listed_test_count": sum(target.test_count for target in targets),
        "wall_seconds": time.monotonic() - started,
        "failed_targets": [result.label for result in failed],
        "targets": [
            {
                "label": target.label,
                "key": dataclasses.asdict(target.key),
                "executable_sha256": target.executable_sha256,
                "test_count": target.test_count,
                "returncode": result_by_label[target.label].returncode,
                "wall_seconds": result_by_label[target.label].wall_seconds,
                "log": str(result_by_label[target.label].log_path),
            }
            for target in sorted(targets, key=lambda item: item.label)
        ],
    }
    receipt_path = receipt_dir / "receipt.json"
    _atomic_json(receipt_path, receipt)
    if managed_receipt_root is not None:
        _prune_owned_runs(managed_receipt_root, receipt_dir)
    if args.verbose and not failed:
        for result in sorted(results, key=lambda item: item.label):
            print(f"\n[TEST LOG] {result.label}")
            try:
                sys.stdout.flush()
                sys.stdout.buffer.write(result.log_path.read_bytes())
                sys.stdout.buffer.flush()
            except OSError as error:
                raise SchedulerError(
                    f"could not replay successful log {result.log_path}: {error}"
                ) from error
    print(
        f"parallel Cargo tests: {'FAIL' if failed else 'PASS'} in "
        f"{receipt['wall_seconds']:.2f}s; receipt {receipt_path}",
        flush=True,
    )
    return 1 if failed else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except SchedulerError as error:
        print(f"parallel Cargo tests: REFUSED: {error}", file=sys.stderr)
        raise SystemExit(2) from error
