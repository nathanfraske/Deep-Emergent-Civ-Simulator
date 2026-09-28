#!/usr/bin/env python3
"""Configure repository-scoped caches and bounded CPU work in a fleet job."""
import datetime
import json
import os
from pathlib import Path
import shutil

REPOSITORY = "nathanfraske/Deep-Emergent-Civ-Simulator"
CACHE_IDS = {
    "0244d049-ffcd-4d14-8e8d-7ba774af9d18",
    "6d3f3c4b-197b-46e9-8dff-c92a5c6ffd66",
    "8cd1cad9-8f4d-45b9-83fe-99b065c61e82",
}


def budget(cpus, total, available, elastic=None, now=None):
    """MiB; preserve the fleet's OS reserve and 2 GiB per compiler worker."""
    if elastic is not None:
        observed = datetime.datetime.fromisoformat(elastic["observed_utc"])
        age = (now - observed).total_seconds()
        if not -5 <= age <= 90:
            raise ValueError("Fleet resource observation is stale")
        limit = elastic["memory_high_mib"]
        maximum = elastic["memory_max_mib"]
        if not 1 <= elastic["cpus"] <= 32 or not 0 < limit <= maximum <= 32768:
            raise ValueError("Invalid fleet resource limits")
        cpus = min(cpus, elastic["cpus"])
        total, available = min(total, limit), min(available, limit)
    memory = min(total - 1536, available - 512)
    if cpus < 1 or memory < 1024:
        raise ValueError("Insufficient resources for a build")
    return {
        "CARGO_BUILD_JOBS": str(max(1, min(cpus, memory // 2048))),
        "RAYON_NUM_THREADS": str(max(1, min(cpus, memory // 2048))),
        "NEXTEST_TEST_THREADS": str(max(1, min(cpus // 2, memory // 2048))),
    }


def configure():
    env = os.environ
    if env.get("RUNNER_ENVIRONMENT") != "self-hosted":
        print("GitHub-hosted job: use the existing Actions Rust cache")
        return
    if env.get("GITHUB_REPOSITORY") != REPOSITORY:
        raise ValueError("Unexpected repository")
    root = Path("/var/cache/test-fleet")
    marker = (root / "cache-survival-marker").read_text().strip()
    if not root.is_mount() or marker not in CACHE_IDS:
        raise ValueError("Expected a qualified persistent fleet cache mount")
    base = root / "user/deep-emergent-civ-v1/linux-x64-bookworm"
    values = {"CARGO_HOME": str(base / "cargo"), "RUSTUP_HOME": str(base / "rustup"),
              "CARGO_TARGET_DIR": str(base / "target"), "XDG_CACHE_HOME": str(base / "downloads")}
    for value in values.values():
        path = Path(value)
        # Reject a pre-existing symlink escape before creating any directories.
        if not path.resolve().is_relative_to(root.resolve()):
            raise ValueError("Cache path escaped its disk")
        path.mkdir(parents=True, exist_ok=True)
    free = shutil.disk_usage(base).free
    if free < 12 * 1024 ** 3:
        raise ValueError("Fleet build cache needs 12 GiB free; preserve existing data")
    memory = {k: int(v.split()[0]) // 1024 for k, v in
              (line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())}
    resource_file = Path("/opt/test-fleet/elastic-budget.json")
    elastic = json.loads(resource_file.read_text()) if resource_file.is_file() else None
    if marker == "8cd1cad9-8f4d-45b9-83fe-99b065c61e82" and elastic is None:
        raise ValueError("Laptop must expose its current resource allocation")
    values.update(budget(len(os.sched_getaffinity(0)), memory["MemTotal"], memory["MemAvailable"],
                         elastic, datetime.datetime.now(datetime.timezone.utc)))
    with Path(env["GITHUB_ENV"]).open("a") as stream:
        stream.writelines(f"{key}={value}\n" for key, value in values.items())
    with Path(env["GITHUB_PATH"]).open("a") as stream:
        stream.write(str(base / "cargo/bin") + "\n")
    print(json.dumps({"event": "fleet-civ-cache", "runner": env.get("RUNNER_NAME"),
                      "cache_id": marker, "free_bytes": free, "values": values,
                      "gpu_enabled": False}))


if __name__ == "__main__":
    configure()
