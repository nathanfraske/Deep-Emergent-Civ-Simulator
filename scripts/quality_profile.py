#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
# Licensed under the Apache License, Version 2.0; see LICENSE.

"""Measure the repository quality pipeline without changing its authority.

This diagnostic runner executes the same ordered commands as the Just PR
recipe. It records child resource use, sampled process-tree pressure, per-core
host load, compiler-cache deltas, logs, and artifact growth. Its JSON is a
machine-local performance receipt. It is never a merge or scientific receipt.
"""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import json
import os
from pathlib import Path
import platform
import re
import resource
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterable, Mapping, Sequence


ROOT = Path(__file__).resolve().parent.parent
CLK_TCK = os.sysconf("SC_CLK_TCK")
PAGE_SIZE = os.sysconf("SC_PAGE_SIZE")
PROFILE_MARKER = ".civsim-quality-profile-run-v1"
MAX_RETAINED_PROFILES = 32
MAX_PROFILE_AGE_SECONDS = 30 * 24 * 60 * 60
MIN_COUNT_PRUNE_AGE_SECONDS = 60 * 60
CANONICAL_PACKAGES = (
    "civsim-core",
    "civsim-ledger",
    "civsim-units",
    "civsim-physics",
    "civsim-materials",
    "civsim-world",
    "civsim-planet-substrate",
    "civsim-planet",
    "civsim-viewer",
    "civsim-gpu",
    "civsim-stone0",
)


@dataclasses.dataclass(frozen=True)
class Stage:
    stage_id: str
    description: str
    command: tuple[str, ...]
    compiler_cache: bool = False


def _cargo(*arguments: str) -> tuple[str, ...]:
    return ("bash", "scripts/cargo_dev.sh", *arguments)


def _package_arguments() -> tuple[str, ...]:
    return tuple(part for package in CANONICAL_PACKAGES for part in ("-p", package))


def pr_stages(tier: str) -> tuple[Stage, ...]:
    packages = _package_arguments()
    return (
        Stage("format", "Canonical formatting check", ("just", "fmt-check"), True),
        Stage(
            "gate-runner-canaries",
            "Declarative runner canaries",
            ("python3", "scripts/gate_runner.py", "--self-test"),
        ),
        Stage(
            "gate-runner-tests",
            "Declarative runner regression tests",
            ("python3", "scripts/test_gate_runner.py"),
        ),
        Stage(
            "target-gc-tests",
            "Build-artifact collector regression tests",
            ("bash", "scripts/test_target_gc.sh"),
        ),
        Stage(
            "structural-pre",
            "Pre-phase structural gates",
            (
                "python3",
                "scripts/gate_runner.py",
                "run",
                "--tier",
                tier,
                "--phase",
                "pre",
            ),
        ),
        Stage(
            "detector-self-tests",
            "Declared detector self-tests",
            (
                "python3",
                "scripts/gate_runner.py",
                "self-tests",
                "--tier",
                tier,
            ),
        ),
        Stage(
            "stone0-self-test",
            "Stone 0 native canaries",
            _cargo(
                "run",
                "-q",
                "-p",
                "civsim-stone0",
                "--bin",
                "stone0-gate",
                "--",
                "--self-test",
            ),
            True,
        ),
        Stage(
            "stone0-ci",
            "Stone 0 canonical structural and provenance route",
            _cargo(
                "run",
                "-q",
                "-p",
                "civsim-stone0",
                "--bin",
                "stone0-gate",
                "--",
                "--ci",
            ),
            True,
        ),
        Stage(
            "structural-post",
            "Post-phase generated inventory gates",
            (
                "python3",
                "scripts/gate_runner.py",
                "run",
                "--tier",
                tier,
                "--phase",
                "post",
            ),
            True,
        ),
        Stage("clippy", "Warnings-denied Clippy", ("just", "lint"), True),
        Stage("tests", "Canonical all-target tests", ("just", "test"), True),
        Stage(
            "rustdoc",
            "Private-item Rustdoc",
            _cargo(
                "doc",
                *packages,
                "--no-deps",
                "--document-private-items",
            ),
            True,
        ),
        Stage(
            "doctests",
            "Canonical doctests",
            _cargo("test", *packages, "--doc"),
            True,
        ),
    )


def fast_stages() -> tuple[Stage, ...]:
    packages = _package_arguments()
    return (
        Stage("format", "Canonical formatting check", ("just", "fmt-check"), True),
        Stage(
            "gate-runner-canaries",
            "Declarative runner canaries",
            ("python3", "scripts/gate_runner.py", "--self-test"),
        ),
        Stage(
            "gate-runner-tests",
            "Declarative runner regression tests",
            ("python3", "scripts/test_gate_runner.py"),
        ),
        Stage(
            "target-gc-tests",
            "Build-artifact collector regression tests",
            ("bash", "scripts/test_target_gc.sh"),
        ),
        Stage(
            "structural-pre",
            "Stop pre-phase structural gates",
            (
                "python3",
                "scripts/gate_runner.py",
                "run",
                "--tier",
                "stop",
                "--phase",
                "pre",
            ),
        ),
        Stage(
            "cargo-check",
            "Canonical all-target compile check",
            _cargo("check", *packages, "--all-targets"),
            True,
        ),
        Stage(
            "structural-post",
            "Stop post-phase generated inventory gates",
            (
                "python3",
                "scripts/gate_runner.py",
                "run",
                "--tier",
                "stop",
                "--phase",
                "post",
            ),
            True,
        ),
    )


def serial_test_stages() -> tuple[Stage, ...]:
    return (
        Stage(
            "tests-serial",
            "Independent Cargo serial all-target cross-check",
            ("just", "test-serial"),
            True,
        ),
    )


def _usage_snapshot() -> resource.struct_rusage:
    return resource.getrusage(resource.RUSAGE_CHILDREN)


def _usage_delta(
    before: resource.struct_rusage, after: resource.struct_rusage
) -> dict[str, float | int]:
    return {
        "user_seconds": after.ru_utime - before.ru_utime,
        "system_seconds": after.ru_stime - before.ru_stime,
        "minor_page_faults": after.ru_minflt - before.ru_minflt,
        "major_page_faults": after.ru_majflt - before.ru_majflt,
        "filesystem_inputs": after.ru_inblock - before.ru_inblock,
        "filesystem_outputs": after.ru_oublock - before.ru_oublock,
        "voluntary_context_switches": after.ru_nvcsw - before.ru_nvcsw,
        "involuntary_context_switches": after.ru_nivcsw - before.ru_nivcsw,
    }


def _cpu_snapshot() -> dict[str, tuple[int, int]]:
    snapshot: dict[str, tuple[int, int]] = {}
    try:
        lines = Path("/proc/stat").read_text(encoding="ascii").splitlines()
    except OSError:
        return snapshot
    for line in lines:
        fields = line.split()
        if not fields or not fields[0].startswith("cpu") or fields[0] == "cpu":
            continue
        try:
            values = [int(value) for value in fields[1:]]
        except ValueError:
            continue
        idle = sum(values[index] for index in (3, 4) if index < len(values))
        total = sum(values)
        snapshot[fields[0]] = (total - idle, total)
    return snapshot


def _cpu_delta(
    before: Mapping[str, tuple[int, int]], after: Mapping[str, tuple[int, int]]
) -> list[dict[str, float | str]]:
    result: list[dict[str, float | str]] = []
    for cpu in sorted(set(before) & set(after), key=lambda item: int(item[3:])):
        busy_before, total_before = before[cpu]
        busy_after, total_after = after[cpu]
        total_delta = total_after - total_before
        busy_delta = busy_after - busy_before
        if total_delta <= 0:
            continue
        result.append(
            {
                "cpu": cpu,
                "busy_percent": 100.0 * busy_delta / total_delta,
            }
        )
    return result


def _read_process_stat(pid: int) -> tuple[int, int, int, int, int, str] | None:
    try:
        raw = Path(f"/proc/{pid}/stat").read_text(encoding="ascii")
    except (OSError, UnicodeError):
        return None
    close = raw.rfind(")")
    if close < 0:
        return None
    command_name = raw[raw.find("(") + 1 : close]
    fields = raw[close + 2 :].split()
    try:
        session = int(fields[3])
        ticks = int(fields[11]) + int(fields[12])
        threads = int(fields[17])
        start_ticks = int(fields[19])
        rss_bytes = int(fields[21]) * PAGE_SIZE
    except (IndexError, ValueError):
        return None
    return session, ticks, threads, rss_bytes, start_ticks, command_name


def _read_process_io(pid: int) -> tuple[int, int]:
    values: dict[str, int] = {}
    try:
        lines = Path(f"/proc/{pid}/io").read_text(encoding="ascii").splitlines()
    except (OSError, UnicodeError):
        return 0, 0
    for line in lines:
        key, separator, raw = line.partition(":")
        if not separator:
            continue
        try:
            values[key] = int(raw.strip())
        except ValueError:
            continue
    return values.get("read_bytes", 0), values.get("write_bytes", 0)


@dataclasses.dataclass
class SampledProcess:
    pid: int
    start_ticks: int
    command: str
    first_seen: float
    last_seen: float
    cpu_ticks: int = 0
    peak_rss_bytes: int = 0
    peak_threads: int = 0


def _process_command(pid: int, fallback: str) -> str:
    try:
        raw = Path(f"/proc/{pid}/cmdline").read_bytes()
        command = " ".join(
            part.decode("utf-8", errors="replace") for part in raw.split(b"\0") if part
        )
    except OSError:
        command = fallback
    if not command:
        command = fallback
    return command[:1000]


@dataclasses.dataclass
class ProcessTreeSampler:
    session_id: int
    prior_ticks: dict[int, int] = dataclasses.field(default_factory=dict)
    sampled_cpu_seconds: float = 0.0
    peak_cpu_cores: float = 0.0
    peak_rss_bytes: int = 0
    peak_processes: int = 0
    peak_threads: int = 0
    max_read_bytes: int = 0
    max_write_bytes: int = 0
    affinity: set[int] = dataclasses.field(default_factory=set)
    process_records: dict[tuple[int, int], SampledProcess] = dataclasses.field(
        default_factory=dict
    )
    _last_sample_time: float | None = None

    def sample(self) -> None:
        now = time.monotonic()
        current_ticks: dict[int, int] = {}
        rss = 0
        threads = 0
        read_bytes = 0
        write_bytes = 0
        processes = 0
        for entry in Path("/proc").iterdir():
            if not entry.name.isdigit():
                continue
            pid = int(entry.name)
            process = _read_process_stat(pid)
            if process is None:
                continue
            session, ticks, process_threads, process_rss, start_ticks, command_name = process
            if session != self.session_id:
                continue
            processes += 1
            current_ticks[pid] = ticks
            rss += process_rss
            threads += process_threads
            process_read, process_write = _read_process_io(pid)
            read_bytes += process_read
            write_bytes += process_write
            try:
                self.affinity.update(os.sched_getaffinity(pid))
            except (AttributeError, OSError):
                pass
            identity = (pid, start_ticks)
            record = self.process_records.get(identity)
            if record is None:
                record = SampledProcess(
                    pid=pid,
                    start_ticks=start_ticks,
                    command=_process_command(pid, command_name),
                    first_seen=now,
                    last_seen=now,
                )
                self.process_records[identity] = record
            record.last_seen = now
            record.cpu_ticks = max(record.cpu_ticks, ticks)
            record.peak_rss_bytes = max(record.peak_rss_bytes, process_rss)
            record.peak_threads = max(record.peak_threads, process_threads)

        tick_delta = 0
        for pid, ticks in current_ticks.items():
            tick_delta += max(0, ticks - self.prior_ticks.get(pid, 0))
        cpu_seconds = tick_delta / CLK_TCK
        self.sampled_cpu_seconds += cpu_seconds
        if self._last_sample_time is not None:
            elapsed = now - self._last_sample_time
            if elapsed > 0:
                self.peak_cpu_cores = max(self.peak_cpu_cores, cpu_seconds / elapsed)
        self._last_sample_time = now
        self.prior_ticks = current_ticks
        self.peak_rss_bytes = max(self.peak_rss_bytes, rss)
        self.peak_processes = max(self.peak_processes, processes)
        self.peak_threads = max(self.peak_threads, threads)
        self.max_read_bytes = max(self.max_read_bytes, read_bytes)
        self.max_write_bytes = max(self.max_write_bytes, write_bytes)

    def top_processes(self, limit: int = 100) -> list[dict[str, Any]]:
        ordered = sorted(
            self.process_records.values(),
            key=lambda record: (record.cpu_ticks, record.last_seen - record.first_seen),
            reverse=True,
        )
        return [
            {
                "pid": record.pid,
                "command": record.command,
                "sampled_wall_seconds": max(0.0, record.last_seen - record.first_seen),
                "sampled_cpu_seconds": record.cpu_ticks / CLK_TCK,
                "peak_rss_bytes": record.peak_rss_bytes,
                "peak_threads": record.peak_threads,
            }
            for record in ordered[:limit]
        ]


def _sccache_stats() -> dict[str, Any] | None:
    if shutil.which("sccache") is None:
        return None
    try:
        result = subprocess.run(
            ["sccache", "--show-stats", "--stats-format", "json"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=10,
            check=True,
        )
        decoded = json.loads(result.stdout)
    except (OSError, subprocess.SubprocessError, UnicodeError, json.JSONDecodeError):
        return None
    stats = decoded.get("stats", {})
    hits = stats.get("cache_hits", {}).get("counts", {})
    misses = stats.get("cache_misses", {}).get("counts", {})
    return {
        "compile_requests": int(stats.get("compile_requests", 0)),
        "cache_hits": sum(int(value) for value in hits.values()),
        "cache_misses": sum(int(value) for value in misses.values()),
        "cache_writes": int(stats.get("cache_writes", 0)),
        "cache_errors": sum(
            int(value)
            for value in stats.get("cache_errors", {}).get("counts", {}).values()
        ),
        "cache_size": decoded.get("cache_size"),
        "max_cache_size": decoded.get("max_cache_size"),
    }


def _counter_delta(
    before: Mapping[str, Any] | None, after: Mapping[str, Any] | None
) -> dict[str, int] | None:
    if before is None or after is None:
        return None
    keys = (
        "compile_requests",
        "cache_hits",
        "cache_misses",
        "cache_writes",
        "cache_errors",
    )
    return {
        key: int(after.get(key, 0)) - int(before.get(key, 0)) for key in keys
    }


def _directory_bytes(path: Path | None) -> int | None:
    if path is None or not path.exists():
        return None
    try:
        result = subprocess.run(
            ["du", "-sb", str(path)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=60,
            check=True,
        )
        return int(result.stdout.split()[0])
    except (OSError, subprocess.SubprocessError, UnicodeError, ValueError, IndexError):
        return None


def _git_output(*arguments: str) -> str | None:
    try:
        result = subprocess.run(
            ["git", *arguments],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=10,
            check=True,
        )
    except (OSError, subprocess.SubprocessError, UnicodeError):
        return None
    return result.stdout.strip()


def _memory_total_bytes() -> int | None:
    return _memory_snapshot().get("MemTotal")


def _memory_snapshot() -> dict[str, int]:
    wanted = {
        "MemTotal",
        "MemFree",
        "MemAvailable",
        "Buffers",
        "Cached",
        "SwapCached",
        "Dirty",
        "Writeback",
        "SReclaimable",
    }
    result: dict[str, int] = {}
    try:
        lines = Path("/proc/meminfo").read_text(encoding="ascii").splitlines()
    except (OSError, UnicodeError):
        return result
    for line in lines:
        key, separator, raw = line.partition(":")
        if not separator or key not in wanted:
            continue
        fields = raw.split()
        try:
            value = int(fields[0])
        except (IndexError, ValueError):
            continue
        multiplier = 1024 if len(fields) > 1 and fields[1] == "kB" else 1
        result[key] = value * multiplier
    return result


def _mapping_delta(
    before: Mapping[str, int], after: Mapping[str, int]
) -> dict[str, int]:
    return {
        key: after[key] - before[key]
        for key in sorted(set(before) & set(after))
    }


def _pressure_snapshot() -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for resource_name in ("cpu", "io", "memory"):
        try:
            lines = Path(f"/proc/pressure/{resource_name}").read_text(
                encoding="ascii"
            ).splitlines()
        except (OSError, UnicodeError):
            continue
        resource: dict[str, int] = {}
        for line in lines:
            fields = line.split()
            if not fields:
                continue
            total = next(
                (field.partition("=")[2] for field in fields[1:] if field.startswith("total=")),
                None,
            )
            try:
                resource[fields[0]] = int(total) if total is not None else 0
            except ValueError:
                continue
        if resource:
            result[resource_name] = resource
    return result


def _pressure_delta(
    before: Mapping[str, Mapping[str, int]],
    after: Mapping[str, Mapping[str, int]],
) -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for resource_name in sorted(set(before) & set(after)):
        delta = _mapping_delta(before[resource_name], after[resource_name])
        if delta:
            result[resource_name] = delta
    return result


def _size_bytes(raw: str) -> int | None:
    match = re.fullmatch(r"(\d+)([KMG]?)", raw.strip(), re.IGNORECASE)
    if match is None:
        return None
    multipliers = {"": 1, "K": 1024, "M": 1024**2, "G": 1024**3}
    return int(match.group(1)) * multipliers[match.group(2).upper()]


def _cache_topology() -> list[dict[str, Any]]:
    instances: dict[tuple[int, str, str], int] = {}
    for cpu_dir in sorted(Path("/sys/devices/system/cpu").glob("cpu[0-9]*")):
        for index in (cpu_dir / "cache").glob("index*"):
            try:
                level = int((index / "level").read_text(encoding="ascii").strip())
                cache_type = (index / "type").read_text(encoding="ascii").strip().lower()
                shared = (index / "shared_cpu_list").read_text(
                    encoding="ascii"
                ).strip()
                size = _size_bytes((index / "size").read_text(encoding="ascii"))
            except (OSError, UnicodeError, ValueError):
                continue
            if size is None:
                continue
            instances[(level, cache_type, shared)] = size
    grouped: dict[tuple[int, str], list[tuple[str, int]]] = {}
    for (level, cache_type, shared), size in instances.items():
        grouped.setdefault((level, cache_type), []).append((shared, size))
    return [
        {
            "level": level,
            "type": cache_type,
            "instances": len(records),
            "bytes_total": sum(size for _, size in records),
            "bytes_per_instance": sorted({size for _, size in records}),
            "shared_cpu_lists": sorted(shared for shared, _ in records),
        }
        for (level, cache_type), records in sorted(grouped.items())
    ]


def _hardware_counter_status() -> str:
    if not Path("/sys/bus/event_source/devices/cpu").exists():
        return "unavailable:no-cpu-pmu"
    if shutil.which("perf") is None:
        return "unavailable:perf-not-installed"
    return "available:not-collected-by-default"


def _windows_host_pmu_sources() -> list[str]:
    executable = shutil.which("wpr.exe")
    if executable is None:
        return []
    try:
        output = subprocess.run(
            [executable, "-pmcsources"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
            timeout=10,
            check=True,
        ).stdout
    except (OSError, subprocess.SubprocessError, UnicodeError):
        return []
    sources: list[str] = []
    for line in output.splitlines():
        match = re.match(r"^\s*\d+\s+([A-Za-z][A-Za-z0-9_]*)\s+\d+", line)
        if match is not None:
            sources.append(match.group(1))
    return sources


def _host_record() -> dict[str, Any]:
    affinity = sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else []
    model = None
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
            if line.lower().startswith("model name"):
                model = line.partition(":")[2].strip()
                break
    except (OSError, UnicodeError):
        pass
    return {
        "platform": platform.platform(),
        "python": platform.python_version(),
        "cpu_model": model,
        "logical_cpus_visible": os.cpu_count(),
        "affinity_cpus": affinity,
        "affinity_count": len(affinity),
        "memory_total_bytes": _memory_total_bytes(),
        "cache_topology": _cache_topology(),
        "perf_hardware_counters": _hardware_counter_status(),
        "windows_host_pmu_sources": _windows_host_pmu_sources(),
    }


def _core_summary(per_core: Sequence[Mapping[str, Any]]) -> dict[str, float] | None:
    loads = [float(record["busy_percent"]) for record in per_core]
    if not loads:
        return None
    return {
        "minimum_percent": min(loads),
        "median_percent": statistics.median(loads),
        "maximum_percent": max(loads),
        "mean_percent": statistics.fmean(loads),
    }


def _log_counts(path: Path) -> dict[str, int]:
    counts = {"gate_cached": 0, "gate_passed": 0, "cargo_finished": 0}
    try:
        with path.open(encoding="utf-8", errors="replace") as handle:
            for line in handle:
                stripped = line.lstrip()
                if stripped.startswith("[CACHED]"):
                    counts["gate_cached"] += 1
                elif stripped.startswith("[PASS]"):
                    counts["gate_passed"] += 1
                if "Finished `" in line or "Finished " in line and " target(s)" in line:
                    counts["cargo_finished"] += 1
    except OSError:
        pass
    return counts


def _run_stage(stage: Stage, log_path: Path, sample_seconds: float) -> dict[str, Any]:
    print(f"[PROFILE] START {stage.stage_id}: {stage.description}", flush=True)
    usage_before = _usage_snapshot()
    cpu_before = _cpu_snapshot()
    memory_before = _memory_snapshot()
    pressure_before = _pressure_snapshot()
    cache_before = _sccache_stats() if stage.compiler_cache else None
    started = time.monotonic()
    with log_path.open("wb") as log:
        process = subprocess.Popen(
            stage.command,
            cwd=ROOT,
            env=os.environ.copy(),
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        sampler = ProcessTreeSampler(process.pid)
        while process.poll() is None:
            sampler.sample()
            time.sleep(sample_seconds)
        sampler.sample()
        returncode = process.returncode
    wall = time.monotonic() - started
    usage = _usage_delta(usage_before, _usage_snapshot())
    cpu_seconds = float(usage["user_seconds"]) + float(usage["system_seconds"])
    available = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else 0
    per_core = _cpu_delta(cpu_before, _cpu_snapshot())
    memory_after = _memory_snapshot()
    pressure_after = _pressure_snapshot()
    cache_after = _sccache_stats() if stage.compiler_cache else None
    record: dict[str, Any] = {
        "id": stage.stage_id,
        "description": stage.description,
        "command": list(stage.command),
        "returncode": returncode,
        "wall_seconds": wall,
        "child_resources": usage,
        "average_cpu_cores": cpu_seconds / wall if wall > 0 else 0.0,
        "average_cpu_percent": 100.0 * cpu_seconds / wall if wall > 0 else 0.0,
        "affinity_utilization_percent": (
            100.0 * cpu_seconds / wall / available if wall > 0 and available else None
        ),
        "sampled": {
            "peak_cpu_cores": sampler.peak_cpu_cores,
            "peak_rss_bytes": sampler.peak_rss_bytes,
            "peak_processes": sampler.peak_processes,
            "peak_threads": sampler.peak_threads,
            "read_bytes_high_water": sampler.max_read_bytes,
            "write_bytes_high_water": sampler.max_write_bytes,
            "affinity_cpus": sorted(sampler.affinity),
            "top_processes": sampler.top_processes(),
        },
        "host_per_core": per_core,
        "host_per_core_summary": _core_summary(per_core),
        "host_memory_before_bytes": memory_before,
        "host_memory_after_bytes": memory_after,
        "host_memory_delta_bytes": _mapping_delta(memory_before, memory_after),
        "host_pressure_stall_microseconds": _pressure_delta(
            pressure_before, pressure_after
        ),
        "sccache_delta": _counter_delta(cache_before, cache_after),
        "log": str(log_path),
        "log_counts": _log_counts(log_path),
    }
    status = "PASS" if returncode == 0 else f"FAIL({returncode})"
    print(
        f"[PROFILE] {status} {stage.stage_id}: {wall:.2f}s wall, "
        f"{cpu_seconds:.2f}s CPU, {record['average_cpu_cores']:.2f} average cores, "
        f"{sampler.peak_rss_bytes / (1024 ** 2):.1f} MiB sampled peak RSS",
        flush=True,
    )
    return record


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


def _default_output(pipeline: str) -> Path:
    cache = os.environ.get("CIVSIM_REPO_CACHE_DIR")
    base = Path(cache) if cache else ROOT / ".git" / "civsim-cache"
    timestamp = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    return base / "profiles" / f"{timestamp}-{os.getpid()}-{pipeline}.json"


def _prepare_profile_paths(output: Path, log_dir: Path) -> None:
    if output.exists() or output.is_symlink():
        raise ValueError(f"profile receipt already exists: {output}")
    if log_dir.exists() or log_dir.is_symlink():
        raise ValueError(f"profile log directory already exists: {log_dir}")
    try:
        output.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        log_dir.mkdir(mode=0o700)
        (log_dir / PROFILE_MARKER).write_text(
            PROFILE_MARKER + "\n", encoding="ascii"
        )
    except OSError as error:
        raise ValueError(f"profile output path is unavailable: {error}") from error


def _prune_owned_profiles(root: Path, current_log_dir: Path) -> None:
    try:
        resolved_root = root.resolve(strict=True)
        resolved_current = current_log_dir.resolve(strict=True)
        resolved_current.relative_to(resolved_root)
    except (OSError, ValueError) as error:
        raise ValueError(f"profile retention root is unsafe: {error}") from error
    owned: list[tuple[int, Path]] = []
    try:
        for entry in resolved_root.iterdir():
            if entry.is_symlink() or not entry.is_dir():
                continue
            resolved = entry.resolve(strict=True)
            resolved.relative_to(resolved_root)
            marker = resolved / PROFILE_MARKER
            if (
                marker.is_symlink()
                or not marker.is_file()
                or marker.read_text(encoding="ascii") != PROFILE_MARKER + "\n"
            ):
                continue
            owned.append((resolved.stat().st_mtime_ns, resolved))
    except (OSError, UnicodeError, ValueError) as error:
        raise ValueError(f"profile retention scan failed: {error}") from error
    owned.sort(reverse=True)
    now = time.time()
    for index, (_, path) in enumerate(owned):
        if path == resolved_current:
            continue
        try:
            age = max(0.0, now - path.stat().st_mtime)
        except OSError as error:
            raise ValueError(f"profile retention metadata failed: {error}") from error
        over_count = (
            index >= MAX_RETAINED_PROFILES
            and age >= MIN_COUNT_PRUNE_AGE_SECONDS
        )
        if age < MAX_PROFILE_AGE_SECONDS and not over_count:
            continue
        receipt = resolved_root / f"{path.name}.json"
        if receipt.exists() and (receipt.is_symlink() or not receipt.is_file()):
            continue
        try:
            path.relative_to(resolved_root)
            shutil.rmtree(path)
            receipt.unlink(missing_ok=True)
        except (OSError, ValueError) as error:
            raise ValueError(f"could not prune owned profile {path}: {error}") from error


def _select_stages(
    pipeline: str, tier: str, selected: Iterable[str]
) -> tuple[Stage, ...]:
    if pipeline == "pr":
        stages = pr_stages(tier)
    elif pipeline == "fast":
        stages = fast_stages()
    else:
        stages = serial_test_stages()
    selected_set = set(selected)
    if not selected_set:
        return stages
    unknown = selected_set - {stage.stage_id for stage in stages}
    if unknown:
        raise ValueError(f"unknown stage(s): {', '.join(sorted(unknown))}")
    return tuple(stage for stage in stages if stage.stage_id in selected_set)


def _self_test() -> int:
    with tempfile.TemporaryDirectory(prefix="civsim-quality-profile-") as raw:
        base = Path(raw)
        stage = Stage(
            "synthetic",
            "Synthetic profiler canary",
            (
                sys.executable,
                "-c",
                "import hashlib; hashlib.sha256(b'profile-canary').hexdigest()",
            ),
        )
        record = _run_stage(stage, base / "synthetic.log", 0.01)
        if record["returncode"] != 0 or record["wall_seconds"] <= 0:
            raise AssertionError("synthetic stage did not produce a successful timing")
        output = base / "receipt.json"
        _atomic_json(output, {"schema": "civsim.quality-profile.self-test.v1", "stage": record})
        if json.loads(output.read_text(encoding="utf-8"))["stage"]["id"] != "synthetic":
            raise AssertionError("profile receipt did not round trip")

        retention_root = base / "retention"
        current_output = retention_root / "current.json"
        current_log = current_output.with_suffix("")
        _prepare_profile_paths(current_output, current_log)
        old_time = time.time() - (2 * MIN_COUNT_PRUNE_AGE_SECONDS)
        for index in range(MAX_RETAINED_PROFILES + 1):
            log_dir = retention_root / f"old-{index:02d}"
            log_dir.mkdir()
            (log_dir / PROFILE_MARKER).write_text(
                PROFILE_MARKER + "\n", encoding="ascii"
            )
            (retention_root / f"{log_dir.name}.json").write_text(
                "{}\n", encoding="ascii"
            )
            os.utime(log_dir, (old_time, old_time))
        unowned = retention_root / "unowned"
        unowned.mkdir()
        os.utime(unowned, (old_time, old_time))
        linked_profile = retention_root / "linked"
        linked_profile.symlink_to(unowned, target_is_directory=True)
        _prune_owned_profiles(retention_root, current_log)
        owned_count = sum(
            1
            for path in retention_root.iterdir()
            if path.is_dir()
            and not path.is_symlink()
            and (path / PROFILE_MARKER).is_file()
        )
        if owned_count != MAX_RETAINED_PROFILES:
            raise AssertionError("profile retention did not enforce its bound")
        if not unowned.exists() or not linked_profile.is_symlink():
            raise AssertionError("profile retention touched unowned state")
    print("quality profiler self-test: PASS")
    return 0


def _parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "pipeline",
        nargs="?",
        choices=("pr", "fast", "serial-test"),
        default="pr",
    )
    parser.add_argument("--tier", default="pr")
    parser.add_argument("--stage", action="append", default=[])
    parser.add_argument("--output", type=Path)
    parser.add_argument("--sample-ms", type=int, default=250)
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(sys.argv[1:] if argv is None else argv)
    if args.self_test:
        return _self_test()
    if args.sample_ms < 10 or args.sample_ms > 10_000:
        raise SystemExit("--sample-ms must be from 10 through 10000")
    try:
        stages = _select_stages(args.pipeline, args.tier, args.stage)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    if args.list:
        for stage in stages:
            print(f"{stage.stage_id:24} {' '.join(stage.command)}")
        return 0

    output = (args.output or _default_output(args.pipeline)).expanduser().resolve()
    log_dir = output.with_suffix("")
    try:
        _prepare_profile_paths(output, log_dir)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    managed_profile_root = output.parent if args.output is None else None
    target = Path(os.environ["CARGO_TARGET_DIR"]) if os.environ.get("CARGO_TARGET_DIR") else None
    started_utc = dt.datetime.now(dt.timezone.utc).isoformat()
    started = time.monotonic()
    receipt: dict[str, Any] = {
        "schema": "civsim.quality-profile.v1",
        "authority_effect": "none",
        "pipeline": args.pipeline,
        "tier": args.tier,
        "started_utc": started_utc,
        "repository": {
            "head": _git_output("rev-parse", "HEAD"),
            "branch": _git_output("branch", "--show-current"),
            "status": _git_output("status", "--short"),
        },
        "host": _host_record(),
        "environment": {
            "cargo_target_dir": str(target) if target else None,
            "civsim_gate_jobs": os.environ.get("CIVSIM_GATE_JOBS"),
            "rustc_wrapper": os.environ.get("RUSTC_WRAPPER"),
            "sccache_dir": os.environ.get("SCCACHE_DIR"),
        },
        "initial": {
            "target_bytes": _directory_bytes(target),
            "sccache": _sccache_stats(),
        },
        "stages": [],
    }

    exit_code = 0
    for stage in stages:
        record = _run_stage(stage, log_dir / f"{stage.stage_id}.log", args.sample_ms / 1000)
        receipt["stages"].append(record)
        _atomic_json(output, receipt)
        if record["returncode"] != 0:
            exit_code = int(record["returncode"]) or 1
            break

    receipt["finished_utc"] = dt.datetime.now(dt.timezone.utc).isoformat()
    receipt["summary"] = {
        "wall_seconds": time.monotonic() - started,
        "stage_wall_seconds": sum(
            float(stage["wall_seconds"]) for stage in receipt["stages"]
        ),
        "stage_cpu_seconds": sum(
            float(stage["child_resources"]["user_seconds"])
            + float(stage["child_resources"]["system_seconds"])
            for stage in receipt["stages"]
        ),
        "completed_stages": len(receipt["stages"]),
        "requested_stages": len(stages),
        "exit_code": exit_code,
    }
    receipt["final"] = {
        "target_bytes": _directory_bytes(target),
        "sccache": _sccache_stats(),
    }
    initial_bytes = receipt["initial"]["target_bytes"]
    final_bytes = receipt["final"]["target_bytes"]
    receipt["summary"]["target_growth_bytes"] = (
        final_bytes - initial_bytes
        if isinstance(initial_bytes, int) and isinstance(final_bytes, int)
        else None
    )
    _atomic_json(output, receipt)
    if managed_profile_root is not None:
        try:
            _prune_owned_profiles(managed_profile_root, log_dir)
        except ValueError as error:
            raise SystemExit(str(error)) from error
    print(f"[PROFILE] receipt: {output}")
    print(f"[PROFILE] logs: {log_dir}")
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
