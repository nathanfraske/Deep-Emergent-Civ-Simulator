#!/usr/bin/env python3
"""Validate Stone 0's singleton Cargo build wiring and cross-check its receipt."""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import tomllib
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parent.parent
WATCHDOG = ROOT / "scripts" / "stone0_build_wiring_watchdog.py"
SCHEMA = "civsim.stone0.build-wiring-pair.v1"
CLAIM_ID = "governance.stone0-build-wiring"
PRODUCER = "civsim.stone0.toml-graph-producer.v1"
CHECKER = "civsim.stone0.section-scan-watchdog.v1"
ANCHOR = "crates/stone0-build"
CONSUMERS = ("crates/planet", "crates/planet-substrate")
MARKER_ENV = "CIVSIM_STONE0_GUARD_MARKER"
MARKER_TOKEN = "civsim.stone0.build-guard.v1"
FILES = (
    "Cargo.toml",
    "justfile",
    "crates/stone0-build/Cargo.toml",
    "crates/stone0-build/build.rs",
    "crates/stone0-build/src/lib.rs",
    "crates/stone0/src/lib.rs",
    "crates/planet/Cargo.toml",
    "crates/planet/build.rs",
    "crates/planet-substrate/Cargo.toml",
    "crates/planet-substrate/build.rs",
)


class WiringError(ValueError):
    """The singleton build topology or pair receipt failed closed."""


def _regular(root: pathlib.Path, relative: str) -> pathlib.Path:
    path = root / pathlib.PurePosixPath(relative)
    try:
        resolved = path.resolve(strict=True)
        resolved.relative_to(root.resolve(strict=True))
    except (FileNotFoundError, OSError, ValueError) as error:
        raise WiringError(f"missing or external wiring file: {relative}") from error
    if path.is_symlink() or not resolved.is_file():
        raise WiringError(f"wiring path is not a plain file: {relative}")
    return resolved


def _manifest(root: pathlib.Path, relative: str) -> dict[str, Any]:
    raw = _regular(root, relative).read_bytes()
    try:
        parsed = tomllib.load(io.BytesIO(raw))
    except (tomllib.TOMLDecodeError, UnicodeDecodeError) as error:
        raise WiringError(f"invalid Cargo manifest: {relative}") from error
    if not isinstance(parsed, dict):
        raise WiringError(f"Cargo manifest is not a table: {relative}")
    return parsed


def _rust_function_section(source: str, signature: str) -> str:
    if source.count(signature) != 1:
        raise WiringError(f"Stone 0 source must contain one {signature!r}")
    start = source.index(signature)
    following = source.find("\nfn ", start + len(signature))
    following_public = source.find("\npub fn ", start + len(signature))
    ends = [position for position in (following, following_public) if position >= 0]
    end = min(ends) if ends else len(source)
    return source[start:end]


def _validate_stone0_root_binding(root: pathlib.Path) -> None:
    source = _regular(root, "crates/stone0/src/lib.rs").read_text(encoding="utf-8")
    explicit_route = _rust_function_section(
        source, "pub fn run_at_repository_root("
    )
    validator = _rust_function_section(
        source, "fn canonicalize_and_validate_repository_root("
    )
    git_top_level = _rust_function_section(source, "fn trusted_git_top_level(")
    git_command = _rust_function_section(source, "fn trusted_git_command(")
    git_executable = _rust_function_section(source, "fn trusted_git_executable(")
    unix_git_executable = _rust_function_section(
        source, "fn trusted_unix_git_executable("
    )
    git_executable_from = _rust_function_section(
        source, "fn trusted_git_executable_from("
    )
    executable_shape = _rust_function_section(
        source, "fn root_executable_shape_is_trusted("
    )
    directory_shape = _rust_function_section(
        source, "fn root_directory_shape_is_trusted("
    )

    explicit_call = "canonicalize_and_validate_repository_root(repo_root)"
    if explicit_route.count(explicit_call) != 1:
        raise WiringError("explicit-root entry point does not call the root validator once")

    trusted_call = "let git_root = trusted_git_top_level(&canonical)?;"
    inequality = "if git_root != canonical {"
    success = "Ok(canonical)"
    trusted_position = validator.find(trusted_call)
    inequality_position = validator.find(inequality)
    refusal_position = validator.find("return Err(format!(", inequality_position)
    success_position = validator.find(success, inequality_position)
    if (
        any(
            position < 0
            for position in (
                trusted_position,
                inequality_position,
                refusal_position,
                success_position,
            )
        )
        or not (
            trusted_position
            < inequality_position
            < refusal_position
            < success_position
        )
        or validator.count(trusted_call) != 1
        or validator.count(inequality) != 1
    ):
        raise WiringError(
            "repository-root validator lost its trusted-Git inequality refusal route"
        )

    if git_top_level.count("let mut command = trusted_git_command(root)?;") != 1:
        raise WiringError("trusted Git top-level query bypasses the trusted command route")

    command_steps = (
        "let git = trusted_git_executable()?;",
        "let mut command = Command::new(git);",
        ".env_clear()",
        '.env("PATH", "/usr/bin:/bin")',
    )
    command_positions = [git_command.find(step) for step in command_steps]
    if (
        any(position < 0 for position in command_positions)
        or command_positions != sorted(command_positions)
        or any(git_command.count(step) != 1 for step in command_steps)
    ):
        raise WiringError(
            "trusted Git command lost its rooted executable, cleared environment, or fixed PATH"
        )

    fixed_candidates = (
        "trusted_git_executable_from(&[",
        'Path::new("/usr/bin/git")',
        'Path::new("/usr/lib/git-core/git")',
        'Path::new("/bin/git")',
    )
    if git_executable.count("trusted_unix_git_executable()") != 1:
        raise WiringError("trusted Git platform selector bypasses the Unix trust route")
    candidate_positions = [unix_git_executable.find(step) for step in fixed_candidates]
    if (
        any(position < 0 for position in candidate_positions)
        or candidate_positions != sorted(candidate_positions)
        or any(unix_git_executable.count(step) != 1 for step in fixed_candidates)
        or unix_git_executable.count("Path::new(") != 3
    ):
        raise WiringError("trusted Git fixed-candidate route changed")

    expected_executable_shape = (
        "is_file && uid == 0 && mode & 0o022 == 0 && "
        "mode & 0o6000 == 0 && mode & 0o111 != 0"
    )
    expected_directory_shape = "is_dir && uid == 0 && mode & 0o022 == 0"
    if " ".join(executable_shape.split()).count(expected_executable_shape) != 1:
        raise WiringError("trusted Git executable metadata predicate changed")
    if " ".join(directory_shape.split()).count(expected_directory_shape) != 1:
        raise WiringError("trusted Git ancestry metadata predicate changed")

    trust_steps = (
        "let mut inspected = BTreeSet::new();",
        "let canonical = match fs::canonicalize(candidate)",
        "if !inspected.insert(canonical.clone()) {",
        "let metadata = match fs::metadata(&canonical)",
        "root_executable_shape_is_trusted(",
        "let mut ancestry_is_trusted = true;",
        "for ancestor in canonical.ancestors().skip(1) {",
        "ancestry_is_trusted &= root_directory_shape_is_trusted(",
        "if executable_is_trusted && ancestry_is_trusted {",
        "return Ok(canonical);",
    )
    trust_positions = [git_executable_from.find(step) for step in trust_steps]
    if (
        any(position < 0 for position in trust_positions)
        or trust_positions != sorted(trust_positions)
        or any(git_executable_from.count(step) != 1 for step in trust_steps)
    ):
        raise WiringError(
            "trusted Git candidate proof lost canonical dedup, metadata checks, ancestry checks, or conjunctive acceptance"
        )


def _validate_anchor_sources(root: pathlib.Path) -> None:
    build = _regular(root, f"{ANCHOR}/build.rs").read_text(encoding="utf-8")
    ordered = (
        'let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");',
        "civsim_stone0::emit_cargo_rerun_inputs(&repo_root);",
        "std::fs::remove_file(&marker)",
        "civsim_stone0::run_at_repository_root(civsim_stone0::Mode::Local, &repo_root)",
        "if code != 0",
        "std::fs::write(&marker, MARKER_SOURCE)",
        "cargo:rustc-env=CIVSIM_STONE0_GUARD_MARKER",
    )
    positions: list[int] = []
    for needle in ordered:
        if build.count(needle) != 1:
            raise WiringError(f"anchor build script must contain one {needle!r}")
        positions.append(build.index(needle))
    if positions != sorted(positions):
        raise WiringError("anchor marker may be emitted only after a clean Stone 0 run")
    if "civsim_stone0::run(civsim_stone0::Mode::Local)" in build:
        raise WiringError("anchor retained the ambient repository-root runner")
    if MARKER_TOKEN not in build:
        raise WiringError("anchor build script changed its linkage token")
    _validate_stone0_root_binding(root)

    library = _regular(root, f"{ANCHOR}/src/lib.rs").read_text(encoding="utf-8")
    for needle in (
        f'include!(env!("{MARKER_ENV}"))',
        MARKER_TOKEN,
        "pub fn assert_guard_linked()",
        "generated::token()",
    ):
        if library.count(needle) != 1:
            raise WiringError(f"anchor library must contain one {needle!r}")

    for consumer in CONSUMERS:
        source = _regular(root, f"{consumer}/build.rs").read_text(encoding="utf-8")
        if source.count("civsim_stone0_build::assert_guard_linked();") != 1:
            raise WiringError(f"{consumer} must call one shared guard sentinel")
        for forbidden in ("civsim_stone0::", "Mode::", "emit_cargo_rerun_inputs"):
            if forbidden in source:
                raise WiringError(f"{consumer} retains a duplicate Stone 0 owner")


def validate(root: pathlib.Path) -> None:
    workspace = _manifest(root, "Cargo.toml").get("workspace")
    if not isinstance(workspace, dict):
        raise WiringError("root workspace table is absent")
    members = workspace.get("members")
    defaults = workspace.get("default-members")
    if not isinstance(members, list) or members.count(ANCHOR) != 1:
        raise WiringError("workspace must contain exactly one Stone 0 build anchor")
    if not isinstance(defaults, list) or ANCHOR in defaults:
        raise WiringError("Stone 0 build anchor must not be a default member")

    anchor = _manifest(root, f"{ANCHOR}/Cargo.toml")
    package = anchor.get("package")
    if not isinstance(package, dict) or package.get("name") != "civsim-stone0-build":
        raise WiringError("anchor package identity changed")
    if package.get("build") != "build.rs":
        raise WiringError("anchor must explicitly require build.rs")
    if "dependencies" in anchor:
        raise WiringError("anchor may not have a runtime dependency table")
    if anchor.get("build-dependencies") != {
        "civsim-stone0": {"path": "../stone0"}
    }:
        raise WiringError("anchor must have exactly one Stone 0 build dependency")

    for consumer in CONSUMERS:
        manifest = _manifest(root, f"{consumer}/Cargo.toml")
        consumer_package = manifest.get("package")
        if not isinstance(consumer_package, dict) or consumer_package.get("build") != "build.rs":
            raise WiringError(f"{consumer} must explicitly require build.rs")
        if manifest.get("build-dependencies") != {
            "civsim-stone0-build": {"path": "../stone0-build"}
        }:
            raise WiringError(f"{consumer} must depend only on the shared build guard")

    justfile = _regular(root, "justfile").read_text(encoding="utf-8")
    assignment = next(
        (line for line in justfile.splitlines() if line.startswith("canonical_packages :=")),
        None,
    )
    if assignment is None or "civsim-stone0-build" in assignment:
        raise WiringError("canonical package aggregate must not select the build anchor")
    _validate_anchor_sources(root)


def receipt(root: pathlib.Path) -> bytes:
    validate(root)
    observed = [
        {
            "path": relative,
            "sha256": hashlib.sha256(_regular(root, relative).read_bytes()).hexdigest(),
        }
        for relative in sorted(FILES)
    ]
    payload = {
        "anchor": ANCHOR,
        "claim_id": CLAIM_ID,
        "consumers": list(CONSUMERS),
        "files": observed,
        "marker_environment": MARKER_ENV,
        "marker_token": MARKER_TOKEN,
        "producer_implementation": PRODUCER,
        "schema": SCHEMA,
        "watchdog_implementation": CHECKER,
    }
    return json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("ascii")


def _fixture() -> tuple[tempfile.TemporaryDirectory[str], pathlib.Path]:
    temporary = tempfile.TemporaryDirectory(prefix="stone0-wiring-producer-")
    root = pathlib.Path(temporary.name)
    for relative in FILES:
        destination = root / pathlib.PurePosixPath(relative)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / pathlib.PurePosixPath(relative), destination)
    return temporary, root


def self_test() -> None:
    temporary, root = _fixture()
    try:
        receipt(root)
        mutations = (
            ("Cargo.toml", ANCHOR, "crates/stone0-build-missing"),
            (f"{ANCHOR}/Cargo.toml", "../stone0", "../not-stone0"),
            (
                f"{ANCHOR}/build.rs",
                "Mode::Local, &repo_root)",
                'Mode::Local, &Path::new("."))',
            ),
            (
                "crates/stone0/src/lib.rs",
                "let git_root = trusted_git_top_level(&canonical)?;",
                "let git_root = canonical.clone();",
            ),
            (
                "crates/stone0/src/lib.rs",
                "if git_root != canonical {",
                "",
            ),
            (
                "crates/stone0/src/lib.rs",
                ".env_clear()",
                '.env_remove("GIT_DIR")',
            ),
            (
                "crates/stone0/src/lib.rs",
                'Path::new("/usr/lib/git-core/git"),',
                'Path::new("/tmp/git"),',
            ),
            (
                "crates/stone0/src/lib.rs",
                "is_file && uid == 0 && mode & 0o022 == 0 && mode & 0o6000 == 0 && mode & 0o111 != 0",
                "is_file && uid == uid && mode & 0o022 == 0 && mode & 0o6000 == 0 && mode & 0o111 != 0",
            ),
            (
                "crates/stone0/src/lib.rs",
                "is_dir && uid == 0 && mode & 0o022 == 0",
                "is_dir && uid == 0 && mode & 0o002 == 0",
            ),
            (
                "crates/stone0/src/lib.rs",
                "if !inspected.insert(canonical.clone()) {",
                "if false {",
            ),
            (
                "crates/stone0/src/lib.rs",
                "if executable_is_trusted && ancestry_is_trusted {\n            return Ok(canonical);",
                "if executable_is_trusted || ancestry_is_trusted {\n            return Ok(canonical);",
            ),
            (f"{ANCHOR}/build.rs", "if code != 0", "if false"),
            (f"{ANCHOR}/src/lib.rs", MARKER_ENV, "UNBOUND_MARKER"),
            ("crates/planet/build.rs", "assert_guard_linked", "guard_was_skipped"),
        )
        for relative, old, new in mutations:
            path = root / pathlib.PurePosixPath(relative)
            held = path.read_text(encoding="utf-8")
            path.write_text(held.replace(old, new, 1), encoding="utf-8")
            try:
                validate(root)
            except WiringError:
                pass
            else:
                raise AssertionError(
                    f"producer wiring canary survived: {relative}: {old}"
                )
            path.write_text(held, encoding="utf-8")

        result = subprocess.run(
            [sys.executable, str(WATCHDOG), "--self-test"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
        if result.returncode != 0:
            raise AssertionError(result.stdout + result.stderr)
    finally:
        temporary.cleanup()
    print("Stone 0 build wiring pair self-test: PASS")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true")
    options = parser.parse_args()
    try:
        if options.self_test:
            self_test()
            return 0
        producer = receipt(ROOT)
        result = subprocess.run(
            [sys.executable, str(WATCHDOG), "--receipt"],
            cwd=ROOT,
            capture_output=True,
            timeout=60,
            check=False,
        )
        if result.returncode != 0:
            raise WiringError(result.stderr.decode("utf-8", errors="replace").strip())
        checker = result.stdout.rstrip(b"\r\n")
        if producer != checker:
            raise WiringError("producer and watchdog wiring receipts differ")
        print("Stone 0 build wiring pair: PASS")
        print(producer.decode("ascii"))
    except (AssertionError, OSError, UnicodeError, WiringError) as error:
        print(f"Stone 0 build wiring pair: FAIL: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
