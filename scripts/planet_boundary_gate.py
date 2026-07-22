#!/usr/bin/env python3
# Copyright 2026 Nathan M. Fraske
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Seal the canonical planet path and its immutable observer boundary.

The package half of this gate asks Cargo for the all-features dependency graph
and walks outward from the workspace's ``civsim-planet`` package. It rejects a
legacy package by name, the private active-candidate substrate package, and
every reachable package or declared path dependency beneath the workspace's
``parked`` directory. The source half scans Rust owned by the planet package
for the concrete legacy APIs, data paths, and crate references that could
bypass the manifest boundary.

The active-candidate half requires one ``civsim-planet-substrate`` workspace
package with exactly fourteen privately declared raw modules and the sealed
absolute-floor bridge used while typed stage adapters are being migrated. It
verifies that the raw files have left the canonical package, that the candidate
package has no ledger or canonical dependency, and that the canonical package
has only ledger and units runtime dependencies until typed adapters land.

The admission half scans the workspace's ``civsim-ledger`` package. It rejects
a public generic value wrapper and value-binding methods on either ``Ledger``
or ``AbsolutePhysicsFloor`` so accounting ancestry cannot become authority to
pair an admitted ID with an arbitrary magnitude.

The observer half starts from the workspace's ``civsim-viewer`` package. Cargo
may cross its one direct edge to ``civsim-planet``, but that package is treated
as an opaque observation provider: planet's own physics closure is not mistaken
for a viewer dependency. Every other reachable legacy, causal, or parked edge
is rejected. Viewer Rust may receive a sealed ``PlanetObservation``, borrow
``PlanetSnapshot``, and inspect an immutable ``RunReceipt``; it may not
construct, evolve, repair, default, or otherwise supply planet state.

Historical and evidence prose may describe the retired method, but no calibration
token is legal in executable canonical-front-door source. Canonical planet files
receive a narrow scan: ledger tiers and provenance remain legal accounting, while
raw-ledger pipeline parameters, post-admission value binding, world-ledger/profile/
manifest loaders, direct authored or per-world value ingress, and imports from the
quarantined raw-parameter substrate are rejected. The crate root may publicly expose
only the canonical module and canonical re-exports. Run
``--self-test`` to exercise clean, transitive-package, parked-path, planet,
front-door, observer-dependency, observer-source, and admit-the-alien canaries
in isolated temporary Cargo workspaces.
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
from typing import Any, Iterable


ROOT = pathlib.Path(__file__).resolve().parent.parent
LEDGER_PACKAGE = "civsim-ledger"
PLANET_PACKAGE = "civsim-planet"
SUBSTRATE_PACKAGE = "civsim-planet-substrate"
VIEWER_PACKAGE = "civsim-viewer"
PLANET_RUNTIME_PACKAGES = frozenset({LEDGER_PACKAGE, "civsim-units"})
SUBSTRATE_RUNTIME_PACKAGES = frozenset(
    {
        "civsim-core",
        "civsim-materials",
        "civsim-physics",
        "civsim-units",
        "civsim-world",
    }
)
RAW_PLANET_MODULES = (
    "astro",
    "deeptime",
    "flexural_field",
    "geodynamics",
    "geodynamics_surface",
    "giants",
    "moons",
    "planet",
    "planetary_assembly",
    "planetary_system",
    "secular",
    "smallbody",
    "stellar",
    "stellar_evolution",
)
SUBSTRATE_ADAPTER_MODULES = ("absolute_floor",)
PLANET_FORBIDDEN_PACKAGES = frozenset(
    {
        "civsim-bio",
        "civsim-foundation",
        SUBSTRATE_PACKAGE,
        "civsim-sim",
        "civsim-viewer",
    }
)
VIEWER_FORBIDDEN_PACKAGES = frozenset(
    {
        "civsim-bio",
        "civsim-compose",
        "civsim-foundation",
        "civsim-gpu",
        "civsim-materials",
        "civsim-physics",
        SUBSTRATE_PACKAGE,
        "civsim-sim",
        "civsim-units",
        "civsim-world",
    }
)

# These rules are intentionally concrete. Rust comments are removed before scanning, while old manifest,
# profile, and tuning interfaces remain sealed from executable planet source.
PLANET_SOURCE_RULES = (
    ("CalibrationManifest", re.compile(r"\bCalibrationManifest\b")),
    ("CalibrationError", re.compile(r"\bCalibrationError\b")),
    ("reserved.toml", re.compile(r"reserved[.]toml")),
    ("Profile::Calibrated", re.compile(r"\bProfile\s*::\s*Calibrated\b")),
    ("Profile::Development", re.compile(r"\bProfile\s*::\s*Development\b")),
    ("dev_default", re.compile(r"\bdev_default\b")),
    ("dev_fixtures", re.compile(r"\bdev_fixtures\b")),
    ("civsim_bio", re.compile(r"\bcivsim_bio\b")),
    ("civsim_foundation", re.compile(r"\bcivsim_foundation\b")),
    ("civsim_sim", re.compile(r"\bcivsim_sim\b")),
    ("civsim_viewer", re.compile(r"\bcivsim_viewer\b")),
    ("parked/ path", re.compile(r"(?i)(?<![A-Za-z0-9_])parked[/\\]")),
    (
        "authored world-structure selector",
        re.compile(
            r"\b(?:WorldStructure|earth_dev|WorldProfile\s*::\s*grounded|"
            r"EARTH_STRUCTURE|EARTH_DEFAULT)\b"
        ),
    ),
    (
        "Mirror default constructor",
        re.compile(r"\b(?:MIRROR_WORLD_SEED|MIRROR_DEFAULT)\b"),
    ),
    (
        "Sun/solar default constructor",
        re.compile(r"\b(?:local_disk_solar_pin|SUN_DEFAULT|SOLAR_DEFAULT)\b"),
    ),
)

# The canonical runner may consume a validated absolute physics floor. A raw
# Ledger remains useful for catalog accounting, but it is not itself proof that
# a value crossed derive-first, Buckingham-Pi, Gap Law, and Residual Law review.
# These rules therefore name concrete ingress APIs instead of banning Ledger,
# Tier, Provenance, Reference, Residue, or Contingency vocabulary.
PLANET_FRONT_DOOR_SOURCE_RULES = (
    (
        "canonical calibration token",
        re.compile(r"\b\w*calibrat\w*\b", re.IGNORECASE),
    ),
    (
        "canonical floating-point type",
        re.compile(r"\bf(?:32|64)\b"),
    ),
    (
        "quarantined planet substrate import",
        re.compile(
            r"\b(?:use\s+)?crate\s*::\s*(?:astro|deeptime|flexural_field|"
            r"geodynamics(?:_surface)?|giants|moons|planet|planetary_assembly|"
            r"planetary_system|secular|smallbody|stellar(?:_evolution)?)\b"
        ),
    ),
    (
        "raw external substrate import",
        re.compile(r"\bcivsim_(?:physics|materials|world)\s*::"),
    ),
    (
        "raw Ledger pipeline input",
        re.compile(
            r"\bfn\s+(?:run_planet|preflight)\s*\([^)]*"
            r":\s*&?\s*(?:civsim_ledger\s*::\s*)?Ledger(?:Value)?\b",
            re.DOTALL,
        ),
    ),
    (
        "world-ledger ingress",
        re.compile(
            r"\b(?:load|open|parse|read|import)_(?:world_)?ledger\b|"
            r"\bWorldLedger\b|"
            r"\b(?:WorldLedger|Ledger)\s*::\s*"
            r"(?:load|open|parse|read|from_(?:file|path|reader|slice|str))\b|"
            r"\bWorldLedgerLoader\b|"
            r"\b(?:toml|ron|serde_json|serde_yaml)\s*::\s*"
            r"from_(?:reader|slice|str)\s*::\s*<\s*"
            r"(?:civsim_ledger\s*::\s*)?(?:Ledger|WorldLedger)\b|"
            r"\binclude_(?:bytes|str)!\s*\([^)]*"
            r"world[-_]?ledger|"
            r'"--world-ledger"',
            re.IGNORECASE,
        ),
    ),
    (
        "world-profile ingress",
        re.compile(
            r"\b(?:load|open|parse|read|import)_(?:world_)?profile\b|"
            r"\bWorldProfile\b|"
            r"\b(?:WorldProfile|Profile)\s*::\s*"
            r"(?:load|open|parse|read|from_(?:file|path|reader|slice|str))\b|"
            r"\b(?:World)?ProfileLoader\b|"
            r"\binclude_(?:bytes|str)!\s*\([^)]*"
            r"world[-_]?profile|"
            r'"--(?:world-)?profile"',
            re.IGNORECASE,
        ),
    ),
    (
        "world-manifest ingress",
        re.compile(
            r"\b(?:load|open|parse|read|import)_(?:world_)?manifest\b|"
            r"\b(?:WorldManifest|PlanetManifest)\b|"
            r"\b(?:WorldManifest|PlanetManifest|Manifest)\s*::\s*"
            r"(?:load|open|parse|read|from_(?:file|path|reader|slice|str))\b|"
            r"\b(?:World|Planet)?ManifestLoader\b|"
            r"\binclude_(?:bytes|str)!\s*\([^)]*"
            r"world[-_]?manifest|"
            r'"--(?:world-)?manifest"',
            re.IGNORECASE,
        ),
    ),
    (
        "unvalidated ledger value binding",
        re.compile(
            r"\b\w*ledger\w*\s*\.\s*bind\s*\(|"
            r"\bLedger\s*::\s*bind\s*\(",
            re.IGNORECASE,
        ),
    ),
    (
        "post-admission floor value binding",
        re.compile(
            r"\b\w*floor\w*\s*\.\s*"
            r"(?:attach_value|bind|bind_value|pair_value)\s*\(|"
            r"\bAbsolutePhysicsFloor\s*::\s*"
            r"(?:attach_value|bind|bind_value|pair_value)\s*\(",
            re.IGNORECASE,
        ),
    ),
    (
        "generic floor identity lookup",
        re.compile(
            r"\b\w*floor\w*\s*\.\s*"
            r"(?:entry|get|lookup|magnitude|value)(?:_by_id)?\s*\(",
            re.IGNORECASE,
        ),
    ),
    (
        "authored value ingress",
        re.compile(
            r"\b(?:Authored(?:World)?Value|authored_(?:world_)?value|"
            r"(?:admit|bind|insert|load|push|set|supply|with)_authored"
            r"(?:_value)?)\b|"
            r'"--(?:authored|world-authored)-(?:input|parameter|value)"',
            re.IGNORECASE,
        ),
    ),
    (
        "per-world value ingress",
        re.compile(
            r"\b(?:PerWorld(?:Input|Parameters?|Value)s?|"
            r"WorldValue(?:Input|Map|Set)?|"
            r"per_world_(?:input|parameters?|value)s?|"
            r"(?:admit|bind|insert|load|push|set|supply|with)_per_world"
            r"(?:_(?:input|parameters?|value))?)\b|"
            r'"--(?:per-world|world)-(?:input|parameter|value)"',
            re.IGNORECASE,
        ),
    ),
    (
        "absolute-floor validation bypass",
        re.compile(
            r"\b(?:Validated|Admitted)?AbsolutePhysicsFloor\s*::\s*"
            r"(?:assume_valid|from_authored|from_per_world|new_unchecked|unchecked)\b",
        ),
    ),
    (
        "public PlanetRunOutcome enum",
        re.compile(r"\bpub\s+enum\s+PlanetRunOutcome\b"),
    ),
    (
        "public PlanetRunOutcome field",
        re.compile(
            r"\bpub\s+struct\s+PlanetRunOutcome\s*\{"
            r"(?:(?!\}).)*\bpub(?:\([^)]*\))?\s+",
            re.DOTALL,
        ),
    ),
    (
        "public PlanetRunOutcome tuple field",
        re.compile(
            r"\bpub\s+struct\s+PlanetRunOutcome\s*\(\s*"
            r"pub(?:\([^)]*\))?\s+",
            re.DOTALL,
        ),
    ),
    (
        "public PlanetRunOutcome unit constructor",
        re.compile(r"\bpub\s+struct\s+PlanetRunOutcome\s*;"),
    ),
    (
        "public PlanetRunOutcome constructor",
        re.compile(
            r"\bimpl\s+PlanetRunOutcome\s*\{"
            r"(?:(?!^\}).)*?\bpub(?:\([^)]*\))?\s+"
            r"(?:const\s+)?fn\s+\w+(?:\s*<[^>{};]*>)?\s*"
            r"\([^)]*\)\s*->\s*(?:(?!\{).)*"
            r"\b(?:Self|PlanetRunOutcome)\b",
            re.DOTALL | re.MULTILINE,
        ),
    ),
)

# Pre-migration abiotic modules compile and run their tests in the separate
# active-candidate package, but their raw Fixed bundles are not public
# entrypoints. A stage may consume them only after the relevant mechanism moves
# behind a typed canonical adapter, so both crate roots expose no raw module.
PLANET_PUBLIC_SURFACE_SOURCE_RULES = (
    (
        "public noncanonical planet module",
        re.compile(
            r"\bpub\s+mod\s+(?:astro|deeptime|flexural_field|geodynamics(?:_surface)?|"
            r"giants|moons|planet|planetary_assembly|planetary_system|secular|smallbody|"
            r"stellar(?:_evolution)?)\b"
        ),
    ),
    (
        "public noncanonical planet re-export",
        re.compile(
            r"\bpub\s+use\s+(?:crate\s*::\s*)?(?:astro|deeptime|flexural_field|"
            r"geodynamics(?:_surface)?|giants|moons|planet|planetary_assembly|"
            r"planetary_system|secular|smallbody|stellar(?:_evolution)?)\b"
        ),
    ),
)

SUBSTRATE_PUBLIC_SURFACE_SOURCE_RULES = (
    (
        "public raw substrate module",
        re.compile(
            r"\bpub\s+mod\s+(?:absolute_floor|astro|deeptime|flexural_field|geodynamics(?:_surface)?|"
            r"giants|moons|planet|planetary_assembly|planetary_system|secular|smallbody|"
            r"stellar(?:_evolution)?)\b"
        ),
    ),
    (
        "public raw substrate re-export",
        re.compile(
            r"\bpub\s+use\s+(?:crate\s*::\s*)?(?:absolute_floor|astro|deeptime|flexural_field|"
            r"geodynamics(?:_surface)?|giants|moons|planet|planetary_assembly|"
            r"planetary_system|secular|smallbody|stellar(?:_evolution)?)\b"
        ),
    ),
)

# Admission must seal values as well as accounting ancestry. A generic value
# wrapper or binding method lets a caller pair any magnitude with an accounted
# or admitted ID after the receipt checks have finished.
LEDGER_VALUE_AUTHORITY_SOURCE_RULES = (
    (
        "public LedgerValue wrapper",
        re.compile(r"\bpub\s+(?:enum|struct|type)\s+LedgerValue\b"),
    ),
    (
        "Ledger value-binding API",
        re.compile(
            r"\bimpl\s+Ledger\s*\{(?:(?!^\}).)*?\bpub\s+fn\s+"
            r"(?:attach_value|bind|bind_value|pair_value)"
            r"(?:\s*<[^>{};]*>)?\s*\(",
            re.DOTALL | re.MULTILINE,
        ),
    ),
    (
        "AbsolutePhysicsFloor value-binding API",
        re.compile(
            r"\bimpl\s+AbsolutePhysicsFloor\s*\{"
            r"(?:(?!^\}).)*?\bpub\s+fn\s+"
            r"(?:attach_value|bind|bind_value|pair_value)"
            r"(?:\s*<[^>{};]*>)?\s*\(",
            re.DOTALL | re.MULTILINE,
        ),
    ),
)

# The viewer is an adapter over an immutable borrow. These are concrete causal
# interfaces and retired builder names, not broad words such as `derive`,
# `step`, `Earth`, or `Sun`, which are valid in sourced mechanism discussions.
VIEWER_SOURCE_RULES = (
    ("civsim_bio", re.compile(r"\bcivsim_bio\b")),
    ("civsim_compose", re.compile(r"\bcivsim_compose\b")),
    ("civsim_foundation", re.compile(r"\bcivsim_foundation\b")),
    ("civsim_gpu", re.compile(r"\bcivsim_gpu\b")),
    ("civsim_materials", re.compile(r"\bcivsim_materials\b")),
    ("civsim_physics", re.compile(r"\bcivsim_physics\b")),
    ("civsim_sim", re.compile(r"\bcivsim_sim\b")),
    ("civsim_units", re.compile(r"\bcivsim_units\b")),
    ("civsim_world", re.compile(r"\bcivsim_world\b")),
    ("legacy viewer crate", re.compile(r"\bcivsim_viewer_legacy\b")),
    ("parked/ path", re.compile(r"(?i)(?<![A-Za-z0-9_])parked[/\\]")),
    (
        "civsim_planet crate alias",
        re.compile(r"\b(?:use|extern\s+crate)\s+civsim_planet\s+as\b"),
    ),
    (
        "civsim_planet grouped import",
        re.compile(r"\buse\s+civsim_planet\s*::\s*\{"),
    ),
    (
        "observer item alias",
        re.compile(
            r"\buse\s+civsim_planet\s*::\s*"
            r"(?:PlanetObservation|PlanetSnapshot|RunReceipt)\s+as\b"
        ),
    ),
    (
        "non-observer civsim_planet API",
        re.compile(
            r"\bcivsim_planet\s*::\s*(?!(?:PlanetObservation|PlanetSnapshot|RunReceipt)\b)"
        ),
    ),
    ("run_planet", re.compile(r"\brun_planet\b")),
    ("PlanetRunSpec", re.compile(r"\bPlanetRunSpec\b")),
    ("PlanetRunOutcome", re.compile(r"\bPlanetRunOutcome\b")),
    ("planet preflight", re.compile(r"\bpreflight\s*\(")),
    ("readiness_receipt", re.compile(r"\breadiness_receipt\b")),
    (
        "audited_substrate_ledger",
        re.compile(r"\baudited_substrate_ledger\b"),
    ),
    (
        "mutable PlanetSnapshot borrow",
        re.compile(
            r"&\s*mut\s+(?:civsim_planet\s*::\s*)?PlanetSnapshot\b"
        ),
    ),
    (
        "owned PlanetSnapshot input",
        re.compile(
            r"(?<!:):(?!:)\s*(?!&)(?:civsim_planet\s*::\s*)?"
            r"PlanetSnapshot\b"
        ),
    ),
    (
        "owned PlanetSnapshot output",
        re.compile(
            r"->\s*(?:civsim_planet\s*::\s*)?PlanetSnapshot\b"
        ),
    ),
    (
        "PlanetSnapshot construction",
        re.compile(
            r"\bPlanetSnapshot\s*(?:::|\{\s*(?:[A-Za-z_]\w*\s*:|\}))"
        ),
    ),
    (
        "mutable PlanetObservation borrow",
        re.compile(r"&\s*mut\s+(?:civsim_planet\s*::\s*)?PlanetObservation\b"),
    ),
    (
        "PlanetObservation construction",
        re.compile(
            r"\bPlanetObservation\s*(?:::|\{\s*(?:[A-Za-z_]\w*\s*:|\}))"
        ),
    ),
    (
        "mutable RunReceipt borrow",
        re.compile(r"&\s*mut\s+(?:civsim_planet\s*::\s*)?RunReceipt\b"),
    ),
    (
        "owned RunReceipt input",
        re.compile(
            r"(?<!:):(?!:)\s*(?!&)"
            r"(?:civsim_planet\s*::\s*)?RunReceipt\b"
        ),
    ),
    (
        "owned RunReceipt output",
        re.compile(r"->\s*(?:civsim_planet\s*::\s*)?RunReceipt\b"),
    ),
    (
        "RunReceipt construction",
        re.compile(r"\bRunReceipt\s*(?:::|\{\s*(?:[A-Za-z_]\w*\s*:|\}))"),
    ),
    (
        "observer type alias",
        re.compile(
            r"\btype\s+[A-Za-z_]\w*(?:\s*<[^;=]*>)?\s*=\s*"
            r"(?:civsim_planet\s*::\s*)?"
            r"(?:PlanetObservation|PlanetSnapshot|RunReceipt)\b",
            re.DOTALL,
        ),
    ),
    (
        "snapshot causal mutation",
        re.compile(
            r"\b(?:planet_)?snapshot\s*\.\s*"
            r"(?:advance|apply|complete|evolve|mutate|repair|run|step|tick|update)\s*\("
        ),
    ),
    (
        "retired derived-viewer builder",
        re.compile(
            r"\b(?:build_globe_fixture|build_deep_time_provinces|"
            r"build_derived_scene(?:_seeded|_with_composition)?|"
            r"build_sampled_planets)\b"
        ),
    ),
    (
        "retired derived-viewer state",
        re.compile(
            r"\b(?:DeepTimeProvinces|DerivedScene|DerivedView|GlobeFixture|"
            r"SampledPlanet)\b"
        ),
    ),
    (
        "retired viewer-side physical derivation",
        re.compile(
            r"\b(?:derive_pre_ms_bolometric_luminosity|"
            r"derive_formation_condensation_temperature|"
            r"derive_isolation_mass_earth|derive_uncompressed_bulk_density|"
            r"derive_core_fraction_and_metal_density|"
            r"derive_crust_shear_modulus_gpa|derive_support_bound_params|"
            r"derive_deep_time_cadence)\b"
        ),
    ),
    (
        "retired viewer-side evolution",
        re.compile(
            r"\b(?:age_provinces_from_young|bombard_tick|"
            r"deep_time_saturation_tick|run_derived|step_deep_time|"
            r"step_provinces)\b"
        ),
    ),
    # Admit-the-alien: reject exact old default and fixture constructors. Plain
    # Earth/Sun reference names remain legal in comments and mechanism labels.
    (
        "Earth default constructor",
        re.compile(
            r"\b(?:earth_dev|WorldProfile\s*::\s*grounded|"
            r"EARTH_STRUCTURE|EARTH_DEFAULT)\b"
        ),
    ),
    (
        "Mirror default constructor",
        re.compile(r"\b(?:MIRROR_WORLD_SEED|MIRROR_DEFAULT)\b"),
    ),
    (
        "Sun/solar default constructor",
        re.compile(r"\b(?:local_disk_solar_pin|SUN_DEFAULT|SOLAR_DEFAULT)\b"),
    ),
    (
        "Earth/Mirror/Sun string fallback",
        re.compile(
            r"(?:unwrap_or(?:_else)?\s*\([^\n;]*|_\s*=>\s*)"
            r'"(?:earth|mirror|sun)"'
        ),
    ),
    (
        "viewer-authored fixed orbit",
        re.compile(
            r"\borbit_au\s*:\s*(?:Fixed\s*::|[-+]?\d)|"
            r"\b(?:const|static)\s+[A-Z0-9_]*(?:ORBIT_AU|SEMI_MAJOR_AXIS)"
        ),
    ),
    (
        "viewer-authored fixed composition",
        re.compile(
            r"\b(?:AbioticSourceRegistry|DiskComposition|ReservedMeltParams|"
            r"SolarAbundances|WorldProfile|WorldgenParams)\s*::|"
            r"\b(?:let\s+(?:mut\s+)?|\w+\s*:\s*)composition\s*=\s*vec!"
        ),
    ),
    (
        "viewer-authored physical fallback",
        re.compile(
            r"\.\s*unwrap_or(?:_else)?\s*\(\s*(?:Fixed\s*::|"
            r"Earth\b|Mirror\b|PlanetSnapshot\b|Sun\b|WorldProfile\b|"
            r"WorldgenParams\b)"
        ),
    ),
    (
        "observer-supplied physical timestep",
        re.compile(
            r"\b(?:const|static|let)\s+(?:mut\s+)?[A-Z0-9_]*"
            r"(?:DEEP_TIME_DT|DT_MYR|PHYSICAL_TIMESTEP|PHYSICS_TIMESTEP|"
            r"TIMESTEP_MYR)[A-Z0-9_]*\b|"
            r"\b(?:deep_time_dt|dt_myr|physical_timestep|physics_timestep|"
            r"timestep_myr)\s*:\s*"
        ),
    ),
)

_RAW_STRING_START = re.compile(r'(?:b|c)?r(#{0,255})"')


class MetadataError(RuntimeError):
    """Cargo could not supply a usable dependency graph."""


def _cargo_metadata(manifest_path: pathlib.Path, *, locked: bool) -> dict[str, Any]:
    """Return Cargo metadata, failing closed on command or JSON errors."""

    cargo = os.environ.get("CARGO", "cargo")
    command = [
        cargo,
        "metadata",
        "--format-version",
        "1",
        "--all-features",
        "--manifest-path",
        str(manifest_path),
    ]
    if locked:
        command.append("--locked")

    environment = os.environ.copy()
    environment["CARGO_TERM_COLOR"] = "never"
    try:
        completed = subprocess.run(
            command,
            cwd=manifest_path.parent,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            env=environment,
        )
    except OSError as error:
        raise MetadataError(f"could not execute {cargo!r}: {error}") from error

    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip() or "no diagnostic"
        raise MetadataError(
            f"cargo metadata exited {completed.returncode} for {manifest_path}: {detail}"
        )
    try:
        metadata = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise MetadataError(f"cargo metadata returned invalid JSON: {error}") from error
    if not isinstance(metadata, dict):
        raise MetadataError("cargo metadata returned a non-object JSON document")
    return metadata


def _normal_path(path: pathlib.Path) -> str:
    """Return a resolved, case-normalized path for containment comparisons."""

    return os.path.normcase(str(path.resolve(strict=False)))


def _is_under(path: pathlib.Path, parent: pathlib.Path) -> bool:
    """Whether path is parent or a descendant, without string-prefix mistakes."""

    candidate = _normal_path(path)
    boundary = _normal_path(parent)
    try:
        return os.path.commonpath((candidate, boundary)) == boundary
    except (OSError, ValueError):
        return False


def _display_path(path: pathlib.Path, workspace_root: pathlib.Path) -> str:
    """Prefer a stable workspace-relative path in diagnostics."""

    try:
        return path.resolve(strict=False).relative_to(
            workspace_root.resolve(strict=False)
        ).as_posix()
    except ValueError:
        return path.resolve(strict=False).as_posix()


def _named_workspace_member(
    metadata: dict[str, Any], package_name: str
) -> tuple[list[str], dict[str, Any] | None, pathlib.Path | None]:
    """Find exactly one named workspace package for a focused source scan."""

    raw_packages = metadata.get("packages")
    raw_members = metadata.get("workspace_members")
    raw_workspace_root = metadata.get("workspace_root")
    if not isinstance(raw_packages, list):
        return (["cargo metadata has no package list"], None, None)
    if not isinstance(raw_members, list):
        return (["cargo metadata has no workspace member list"], None, None)
    if not isinstance(raw_workspace_root, str):
        return (["cargo metadata has no workspace root"], None, None)

    packages = {
        package["id"]: package
        for package in raw_packages
        if isinstance(package, dict) and isinstance(package.get("id"), str)
    }
    member_ids = {member for member in raw_members if isinstance(member, str)}
    matching = sorted(
        package_id
        for package_id in member_ids
        if package_id in packages
        and packages[package_id].get("name") == package_name
    )
    workspace_root = pathlib.Path(raw_workspace_root)
    if not matching:
        return ([f"workspace has no {package_name} member"], None, workspace_root)
    if len(matching) != 1:
        return (
            [
                f"workspace has {len(matching)} {package_name} members; "
                "exactly one is required"
            ],
            None,
            workspace_root,
        )
    return ([], packages[matching[0]], workspace_root)


def _runtime_dependency_violations(
    package: dict[str, Any],
    *,
    package_name: str,
    expected: frozenset[str],
) -> list[str]:
    """Require an exact direct normal-dependency boundary for one package."""

    raw_dependencies = package.get("dependencies")
    if not isinstance(raw_dependencies, list):
        return [f"{package_name} has a malformed declared dependency list"]
    actual: set[str] = set()
    violations: list[str] = []
    for dependency in raw_dependencies:
        if not isinstance(dependency, dict):
            violations.append(f"{package_name} has a malformed declared dependency")
            continue
        if dependency.get("kind") is not None:
            continue
        name = dependency.get("name")
        if not isinstance(name, str):
            violations.append(
                f"{package_name} has a runtime dependency without a package name"
            )
            continue
        actual.add(name)
    for name in sorted(actual - expected):
        violations.append(f"unexpected {package_name} runtime dependency: {name}")
    for name in sorted(expected - actual):
        violations.append(f"missing {package_name} runtime dependency: {name}")
    return violations


def _substrate_layout_violations(
    package: dict[str, Any], workspace_root: pathlib.Path
) -> list[str]:
    """Require exact private ownership of the retained raw modules and floor bridge."""

    violations: list[str] = []
    manifest = pathlib.Path(str(package.get("manifest_path", ""))).resolve(
        strict=False
    )
    expected_manifest = (
        workspace_root / "crates" / "planet-substrate" / "Cargo.toml"
    ).resolve(strict=False)
    if _normal_path(manifest) != _normal_path(expected_manifest):
        violations.append(
            f"{SUBSTRATE_PACKAGE} manifest must be crates/planet-substrate/Cargo.toml, "
            f"found {_display_path(manifest, workspace_root)}"
        )

    source_dir = manifest.parent / "src"
    expected_files = (
        {"lib.rs"}
        | {f"{module}.rs" for module in RAW_PLANET_MODULES}
        | {f"{module}.rs" for module in SUBSTRATE_ADAPTER_MODULES}
    )
    actual_files = (
        {path.name for path in source_dir.glob("*.rs") if path.is_file()}
        if source_dir.is_dir()
        else set()
    )
    for name in sorted(expected_files - actual_files):
        violations.append(f"missing private substrate source: crates/planet-substrate/src/{name}")
    for name in sorted(actual_files - expected_files):
        violations.append(
            f"unexpected pre-adapter substrate source: crates/planet-substrate/src/{name}"
        )

    lib_path = source_dir / "lib.rs"
    try:
        active_lib = _without_rust_comments(lib_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError) as error:
        violations.append(f"cannot read {SUBSTRATE_PACKAGE} crate root: {error}")
        active_lib = ""
    for module in RAW_PLANET_MODULES:
        declaration = re.compile(
            rf"^\s*mod\s+{re.escape(module)}\s*;\s*$", re.MULTILINE
        )
        count = len(declaration.findall(active_lib))
        if count != 1:
            violations.append(
                f"private substrate module declaration {module!r} appears {count} times"
            )

        old_path = workspace_root / "crates" / "planet" / "src" / f"{module}.rs"
        if old_path.is_file():
            violations.append(
                f"raw module remains in canonical package: "
                f"{_display_path(old_path, workspace_root)}"
            )
    for module in SUBSTRATE_ADAPTER_MODULES:
        declaration = re.compile(
            rf"^\s*mod\s+{re.escape(module)}\s*;\s*$", re.MULTILINE
        )
        count = len(declaration.findall(active_lib))
        if count != 1:
            violations.append(
                f"private substrate adapter declaration {module!r} appears {count} times"
            )
    return violations


def _package_name(package_by_id: dict[str, dict[str, Any]], package_id: str) -> str:
    package = package_by_id.get(package_id)
    if package is None:
        return package_id
    return str(package.get("name", package_id))


def _chain_text(
    package_by_id: dict[str, dict[str, Any]],
    parents: dict[str, str | None],
    package_id: str,
) -> str:
    chain: list[str] = []
    cursor: str | None = package_id
    seen: set[str] = set()
    while cursor is not None and cursor not in seen:
        seen.add(cursor)
        chain.append(_package_name(package_by_id, cursor))
        cursor = parents.get(cursor)
    return " -> ".join(reversed(chain))


def _dependency_violations(
    metadata: dict[str, Any],
) -> tuple[list[str], dict[str, Any] | None, pathlib.Path | None]:
    """Inspect the resolved planet closure and return its package for source scans."""

    violations: set[str] = set()
    raw_packages = metadata.get("packages")
    raw_members = metadata.get("workspace_members")
    raw_resolve = metadata.get("resolve")
    raw_workspace_root = metadata.get("workspace_root")
    if not isinstance(raw_packages, list):
        return (["cargo metadata has no package list"], None, None)
    if not isinstance(raw_members, list):
        return (["cargo metadata has no workspace member list"], None, None)
    if not isinstance(raw_resolve, dict) or not isinstance(raw_resolve.get("nodes"), list):
        return (["cargo metadata has no resolved dependency graph"], None, None)
    if not isinstance(raw_workspace_root, str):
        return (["cargo metadata has no workspace root"], None, None)

    workspace_root = pathlib.Path(raw_workspace_root)
    parked_root = workspace_root / "parked"
    package_by_id: dict[str, dict[str, Any]] = {}
    for package in raw_packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str):
            violations.add("cargo metadata contains a package without a string id")
            continue
        package_by_id[package["id"]] = package

    member_ids = {member for member in raw_members if isinstance(member, str)}
    planet_ids = sorted(
        package_id
        for package_id in member_ids
        if package_id in package_by_id
        and package_by_id[package_id].get("name") == PLANET_PACKAGE
    )
    if not planet_ids:
        violations.add(f"workspace has no {PLANET_PACKAGE} member")
        return (sorted(violations), None, workspace_root)
    if len(planet_ids) != 1:
        violations.add(
            f"workspace has {len(planet_ids)} {PLANET_PACKAGE} members; exactly one is required"
        )
        return (sorted(violations), None, workspace_root)

    planet_id = planet_ids[0]
    nodes_by_id: dict[str, dict[str, Any]] = {}
    for node in raw_resolve["nodes"]:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str):
            violations.add("cargo metadata contains a resolve node without a string id")
            continue
        nodes_by_id[node["id"]] = node

    parents: dict[str, str | None] = {planet_id: None}
    queue: collections.deque[str] = collections.deque((planet_id,))
    order: list[str] = []
    while queue:
        package_id = queue.popleft()
        order.append(package_id)
        node = nodes_by_id.get(package_id)
        if node is None:
            violations.add(
                f"resolved graph has no node for {_package_name(package_by_id, package_id)}"
            )
            continue
        dependency_ids: set[str] = set()
        raw_deps = node.get("deps", [])
        if not isinstance(raw_deps, list):
            violations.add(f"resolve node {package_id} has a malformed dependency list")
            continue
        for dependency in raw_deps:
            if not isinstance(dependency, dict) or not isinstance(dependency.get("pkg"), str):
                violations.add(f"resolve node {package_id} has a dependency without a package id")
                continue
            dependency_ids.add(dependency["pkg"])
        for dependency_id in sorted(dependency_ids):
            if dependency_id not in parents:
                parents[dependency_id] = package_id
                queue.append(dependency_id)

    reachable_ids = set(order)
    reachable_names = {
        str(package_by_id[package_id].get("name"))
        for package_id in reachable_ids
        if package_id in package_by_id
    }
    for package_id in order:
        package = package_by_id.get(package_id)
        if package is None:
            violations.add(f"resolved package {package_id} is absent from the package list")
            continue
        name = str(package.get("name", package_id))
        chain = _chain_text(package_by_id, parents, package_id)
        if name in PLANET_FORBIDDEN_PACKAGES:
            violations.add(f"forbidden package in planet dependency closure: {chain}")

        raw_manifest = package.get("manifest_path")
        if not isinstance(raw_manifest, str):
            violations.add(f"package {name} has no manifest path")
        else:
            manifest = pathlib.Path(raw_manifest)
            if _is_under(manifest, parked_root):
                violations.add(
                    "package beneath parked/ in planet dependency closure: "
                    f"{chain} ({_display_path(manifest, workspace_root)})"
                )

        # Cargo's all-features graph should contain every optional edge. Inspect
        # declarations too so a malformed or target-hidden path still fails closed.
        raw_dependencies = package.get("dependencies", [])
        if not isinstance(raw_dependencies, list):
            violations.add(f"package {name} has a malformed declared dependency list")
            continue
        for dependency in raw_dependencies:
            if not isinstance(dependency, dict):
                violations.add(f"package {name} has a malformed declared dependency")
                continue
            dependency_name = dependency.get("name")
            if (
                dependency_name in PLANET_FORBIDDEN_PACKAGES
                and dependency_name not in reachable_names
            ):
                violations.add(
                    f"forbidden declared dependency outside Cargo's resolved closure: "
                    f"{chain} -> {dependency_name}"
                )
            dependency_path = dependency.get("path")
            if isinstance(dependency_path, str) and _is_under(
                pathlib.Path(dependency_path), parked_root
            ):
                violations.add(
                    f"declared dependency path beneath parked/: {chain} -> "
                    f"{dependency_name or '<unnamed>'} "
                    f"({_display_path(pathlib.Path(dependency_path), workspace_root)})"
                )

    return (sorted(violations), package_by_id[planet_id], workspace_root)


def _legacy_package_name(name: str) -> bool:
    lowered = name.casefold()
    return lowered == "legacy" or lowered.startswith("legacy-") or lowered.endswith("-legacy")


def _legacy_package_path(path: pathlib.Path, workspace_root: pathlib.Path) -> bool:
    """Whether a manifest or dependency path crosses a parked/legacy boundary."""

    resolved = path.resolve(strict=False)
    root = workspace_root.resolve(strict=False)
    try:
        relative = resolved.relative_to(root)
    except ValueError:
        return False
    directory_parts = relative.parts[:-1] if relative.suffix else relative.parts
    return any(part.casefold() in {"legacy", "parked"} for part in directory_parts)


def _viewer_dependency_violations(
    metadata: dict[str, Any],
) -> tuple[list[str], dict[str, Any] | None, pathlib.Path | None]:
    """Inspect viewer dependencies while treating planet as an opaque provider."""

    violations: set[str] = set()
    raw_packages = metadata.get("packages")
    raw_members = metadata.get("workspace_members")
    raw_resolve = metadata.get("resolve")
    raw_workspace_root = metadata.get("workspace_root")
    if not isinstance(raw_packages, list):
        return (["cargo metadata has no package list"], None, None)
    if not isinstance(raw_members, list):
        return (["cargo metadata has no workspace member list"], None, None)
    if not isinstance(raw_resolve, dict) or not isinstance(raw_resolve.get("nodes"), list):
        return (["cargo metadata has no resolved dependency graph"], None, None)
    if not isinstance(raw_workspace_root, str):
        return (["cargo metadata has no workspace root"], None, None)

    workspace_root = pathlib.Path(raw_workspace_root)
    package_by_id: dict[str, dict[str, Any]] = {}
    for package in raw_packages:
        if not isinstance(package, dict) or not isinstance(package.get("id"), str):
            violations.add("cargo metadata contains a package without a string id")
            continue
        package_by_id[package["id"]] = package

    member_ids = {member for member in raw_members if isinstance(member, str)}
    viewer_ids = sorted(
        package_id
        for package_id in member_ids
        if package_id in package_by_id
        and package_by_id[package_id].get("name") == VIEWER_PACKAGE
    )
    if not viewer_ids:
        violations.add(f"workspace has no canonical {VIEWER_PACKAGE} member")
        return (sorted(violations), None, workspace_root)
    if len(viewer_ids) != 1:
        violations.add(
            f"workspace has {len(viewer_ids)} {VIEWER_PACKAGE} members; exactly one is required"
        )
        return (sorted(violations), None, workspace_root)

    viewer_id = viewer_ids[0]
    canonical_planet_ids = {
        package_id
        for package_id in member_ids
        if package_id in package_by_id
        and package_by_id[package_id].get("name") == PLANET_PACKAGE
    }
    if len(canonical_planet_ids) != 1:
        violations.add(
            f"viewer boundary requires exactly one workspace {PLANET_PACKAGE} provider"
        )
    canonical_planet_id = (
        next(iter(canonical_planet_ids)) if len(canonical_planet_ids) == 1 else None
    )
    nodes_by_id: dict[str, dict[str, Any]] = {}
    for node in raw_resolve["nodes"]:
        if not isinstance(node, dict) or not isinstance(node.get("id"), str):
            violations.add("cargo metadata contains a resolve node without a string id")
            continue
        nodes_by_id[node["id"]] = node

    viewer_node = nodes_by_id.get(viewer_id)
    if viewer_node is None:
        violations.add(f"resolved graph has no node for {VIEWER_PACKAGE}")
        return (sorted(violations), package_by_id[viewer_id], workspace_root)
    raw_direct_deps = viewer_node.get("deps", [])
    if not isinstance(raw_direct_deps, list):
        violations.add(f"resolve node {VIEWER_PACKAGE} has a malformed dependency list")
        raw_direct_deps = []
    direct_planet_ids = {
        dependency.get("pkg")
        for dependency in raw_direct_deps
        if isinstance(dependency, dict)
        and isinstance(dependency.get("pkg"), str)
        and _package_name(package_by_id, dependency["pkg"]) == PLANET_PACKAGE
    }
    if len(direct_planet_ids) != 1 or direct_planet_ids != canonical_planet_ids:
        violations.add(
            f"{VIEWER_PACKAGE} must depend directly on the workspace's canonical "
            f"{PLANET_PACKAGE} package"
        )

    viewer_package = package_by_id[viewer_id]
    raw_viewer_declarations = viewer_package.get("dependencies", [])
    if isinstance(raw_viewer_declarations, list):
        for dependency in raw_viewer_declarations:
            if (
                isinstance(dependency, dict)
                and dependency.get("name") == PLANET_PACKAGE
                and dependency.get("rename") is not None
            ):
                violations.add(
                    f"{VIEWER_PACKAGE} may not rename its {PLANET_PACKAGE} dependency"
                )

    parents: dict[str, str | None] = {viewer_id: None}
    queue: collections.deque[str] = collections.deque((viewer_id,))
    order: list[str] = []
    while queue:
        package_id = queue.popleft()
        order.append(package_id)
        package_name = _package_name(package_by_id, package_id)
        if package_id == canonical_planet_id:
            # The viewer is allowed to see sealed observation artifacts through this edge. The
            # provider's causal closure belongs to the planet gate, not the observer.
            continue
        node = nodes_by_id.get(package_id)
        if node is None:
            violations.add(f"resolved graph has no node for {package_name}")
            continue
        raw_deps = node.get("deps", [])
        if not isinstance(raw_deps, list):
            violations.add(f"resolve node {package_id} has a malformed dependency list")
            continue
        dependency_ids = {
            dependency["pkg"]
            for dependency in raw_deps
            if isinstance(dependency, dict) and isinstance(dependency.get("pkg"), str)
        }
        for dependency_id in sorted(dependency_ids):
            if dependency_id not in parents:
                parents[dependency_id] = package_id
                queue.append(dependency_id)

    reachable_ids = set(order)
    reachable_names = {
        _package_name(package_by_id, package_id) for package_id in reachable_ids
    }
    for package_id in order:
        package = package_by_id.get(package_id)
        if package is None:
            violations.add(f"resolved package {package_id} is absent from the package list")
            continue
        name = str(package.get("name", package_id))
        chain = _chain_text(package_by_id, parents, package_id)
        if name in VIEWER_FORBIDDEN_PACKAGES:
            violations.add(f"forbidden package in viewer dependency closure: {chain}")
        if name == PLANET_PACKAGE and package_id != canonical_planet_id:
            violations.add(f"noncanonical planet package in viewer dependency closure: {chain}")
        if package_id != viewer_id and _legacy_package_name(name):
            violations.add(f"legacy package in viewer dependency closure: {chain}")

        raw_manifest = package.get("manifest_path")
        if not isinstance(raw_manifest, str):
            violations.add(f"package {name} has no manifest path")
        else:
            manifest = pathlib.Path(raw_manifest)
            if _legacy_package_path(manifest, workspace_root):
                violations.add(
                    "parked/legacy package in viewer dependency closure: "
                    f"{chain} ({_display_path(manifest, workspace_root)})"
                )

        if package_id == canonical_planet_id:
            continue
        raw_dependencies = package.get("dependencies", [])
        if not isinstance(raw_dependencies, list):
            violations.add(f"package {name} has a malformed declared dependency list")
            continue
        for dependency in raw_dependencies:
            if not isinstance(dependency, dict):
                violations.add(f"package {name} has a malformed declared dependency")
                continue
            dependency_name = dependency.get("name")
            forbidden_name = dependency_name in VIEWER_FORBIDDEN_PACKAGES or (
                isinstance(dependency_name, str)
                and _legacy_package_name(dependency_name)
            )
            if forbidden_name and dependency_name not in reachable_names:
                violations.add(
                    "forbidden declared dependency outside Cargo's viewer closure: "
                    f"{chain} -> {dependency_name}"
                )
            dependency_path = dependency.get("path")
            if isinstance(dependency_path, str) and _legacy_package_path(
                pathlib.Path(dependency_path), workspace_root
            ):
                violations.add(
                    f"declared viewer dependency path beneath parked/legacy: {chain} -> "
                    f"{dependency_name or '<unnamed>'} "
                    f"({_display_path(pathlib.Path(dependency_path), workspace_root)})"
                )

    return (sorted(violations), viewer_package, workspace_root)


def _without_rust_comments(text: str) -> str:
    """Replace Rust comments with spaces while preserving strings and line numbers."""

    output: list[str] = []
    index = 0
    block_depth = 0
    while index < len(text):
        if block_depth:
            if text.startswith("/*", index):
                output.extend((" ", " "))
                index += 2
                block_depth += 1
            elif text.startswith("*/", index):
                output.extend((" ", " "))
                index += 2
                block_depth -= 1
            else:
                output.append("\n" if text[index] == "\n" else " ")
                index += 1
            continue

        if text.startswith("//", index):
            end = text.find("\n", index + 2)
            if end < 0:
                output.extend(" " for _ in text[index:])
                break
            output.extend(" " for _ in text[index:end])
            output.append("\n")
            index = end + 1
            continue
        if text.startswith("/*", index):
            output.extend((" ", " "))
            index += 2
            block_depth = 1
            continue

        raw_match = _RAW_STRING_START.match(text, index)
        if raw_match is not None:
            terminator = '"' + raw_match.group(1)
            end = text.find(terminator, raw_match.end())
            if end < 0:
                output.append(text[index:])
                break
            end += len(terminator)
            output.append(text[index:end])
            index = end
            continue

        if text[index] == '"':
            end = index + 1
            while end < len(text):
                if text[end] == "\\":
                    end += 2
                    continue
                end += 1
                if text[end - 1] == '"':
                    break
            output.append(text[index:end])
            index = end
            continue

        output.append(text[index])
        index += 1
    return "".join(output)


def _package_rust_files(package: dict[str, Any]) -> list[pathlib.Path]:
    """Find package-owned Rust, including target roots outside conventional dirs."""

    manifest = pathlib.Path(str(package["manifest_path"]))
    crate_root = manifest.parent
    files: set[pathlib.Path] = set()
    for directory_name in ("src", "tests", "examples", "benches"):
        directory = crate_root / directory_name
        if directory.is_dir():
            files.update(path for path in directory.rglob("*.rs") if path.is_file())
    build_script = crate_root / "build.rs"
    if build_script.is_file():
        files.add(build_script)
    for target in package.get("targets", []):
        if isinstance(target, dict) and isinstance(target.get("src_path"), str):
            path = pathlib.Path(target["src_path"])
            if path.is_file():
                files.add(path)
    return sorted(files, key=lambda path: path.as_posix())


def _is_planet_front_door_source(
    package: dict[str, Any], path: pathlib.Path
) -> bool:
    """Whether Rust is in the canonical pipeline or its command-line door."""

    crate_root = pathlib.Path(str(package["manifest_path"])).parent
    try:
        relative = path.resolve(strict=False).relative_to(
            crate_root.resolve(strict=False)
        )
    except ValueError:
        return False
    parts = relative.parts
    return (
        len(parts) >= 3
        and parts[0:2] == ("src", "canonical")
        and relative.suffix == ".rs"
    ) or parts == ("src", "bin", "run_planet.rs")


def _is_planet_public_surface_source(
    package: dict[str, Any], path: pathlib.Path
) -> bool:
    manifest_dir = pathlib.Path(package["manifest_path"]).resolve().parent
    try:
        relative = path.resolve().relative_to(manifest_dir)
    except ValueError:
        return False
    return relative.parts == ("src", "lib.rs")


def _whole_source_rule_violations(
    active_text: str,
    *,
    package_name: str,
    display: str,
    rules: Iterable[tuple[str, re.Pattern[str]]],
) -> list[str]:
    """Report path-scoped patterns, including signatures split across lines."""

    violations: list[str] = []
    for label, pattern in rules:
        for match in pattern.finditer(active_text):
            line_number = active_text.count("\n", 0, match.start()) + 1
            violations.append(
                f"forbidden {package_name} source token `{label}` "
                f"in {display}:{line_number}"
            )
    return violations


def _source_violations(
    package: dict[str, Any],
    workspace_root: pathlib.Path,
    *,
    package_name: str,
    rules: Iterable[tuple[str, re.Pattern[str]]],
) -> list[str]:
    """Find concrete forbidden source edges in package-owned Rust."""

    violations: list[str] = []
    parked_root = workspace_root / "parked"
    rust_files = _package_rust_files(package)
    if not rust_files:
        return [f"{package_name} has no Rust source to scan"]
    for path in rust_files:
        display = _display_path(path, workspace_root)
        if _is_under(path, parked_root):
            violations.append(f"{package_name} target source is beneath parked/: {display}")
        try:
            source = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            violations.append(f"cannot read {package_name} source {display}: {error}")
            continue
        active_text = _without_rust_comments(source)
        for line_number, line in enumerate(active_text.splitlines(), start=1):
            for label, pattern in rules:
                if pattern.search(line):
                    violations.append(
                        f"forbidden {package_name} source token `{label}` "
                        f"in {display}:{line_number}"
                    )
        if package_name == PLANET_PACKAGE and _is_planet_front_door_source(
            package, path
        ):
            violations.extend(
                _whole_source_rule_violations(
                    active_text,
                    package_name=package_name,
                    display=display,
                    rules=PLANET_FRONT_DOOR_SOURCE_RULES,
                )
            )
        if package_name == PLANET_PACKAGE and _is_planet_public_surface_source(
            package, path
        ):
            violations.extend(
                _whole_source_rule_violations(
                    active_text,
                    package_name=package_name,
                    display=display,
                    rules=PLANET_PUBLIC_SURFACE_SOURCE_RULES,
                )
            )
        if package_name == SUBSTRATE_PACKAGE and _is_planet_public_surface_source(
            package, path
        ):
            violations.extend(
                _whole_source_rule_violations(
                    active_text,
                    package_name=package_name,
                    display=display,
                    rules=SUBSTRATE_PUBLIC_SURFACE_SOURCE_RULES,
                )
            )
        if package_name == LEDGER_PACKAGE:
            violations.extend(
                _whole_source_rule_violations(
                    active_text,
                    package_name=package_name,
                    display=display,
                    rules=LEDGER_VALUE_AUTHORITY_SOURCE_RULES,
                )
            )
        if package_name == VIEWER_PACKAGE:
            violations.extend(
                _whole_source_rule_violations(
                    active_text,
                    package_name=package_name,
                    display=display,
                    rules=VIEWER_SOURCE_RULES,
                )
            )
    return sorted(set(violations))


def check_manifest(manifest_path: pathlib.Path, *, locked: bool = True) -> list[str]:
    """Run both halves of the boundary gate for one workspace manifest."""

    try:
        metadata = _cargo_metadata(manifest_path.resolve(), locked=locked)
    except MetadataError as error:
        return [f"metadata failure: {error}"]
    violations, planet, workspace_root = _dependency_violations(metadata)
    if planet is not None and workspace_root is not None:
        violations.extend(
            _runtime_dependency_violations(
                planet,
                package_name=PLANET_PACKAGE,
                expected=PLANET_RUNTIME_PACKAGES,
            )
        )
        violations.extend(
            _source_violations(
                planet,
                workspace_root,
                package_name=PLANET_PACKAGE,
                rules=PLANET_SOURCE_RULES,
            )
        )
    substrate_violations, substrate, substrate_workspace_root = _named_workspace_member(
        metadata, SUBSTRATE_PACKAGE
    )
    violations.extend(substrate_violations)
    if substrate is not None and substrate_workspace_root is not None:
        violations.extend(
            _runtime_dependency_violations(
                substrate,
                package_name=SUBSTRATE_PACKAGE,
                expected=SUBSTRATE_RUNTIME_PACKAGES,
            )
        )
        violations.extend(
            _substrate_layout_violations(substrate, substrate_workspace_root)
        )
        violations.extend(
            _source_violations(
                substrate,
                substrate_workspace_root,
                package_name=SUBSTRATE_PACKAGE,
                rules=(),
            )
        )
    ledger_violations, ledger, ledger_workspace_root = _named_workspace_member(
        metadata, LEDGER_PACKAGE
    )
    violations.extend(ledger_violations)
    if ledger is not None and ledger_workspace_root is not None:
        violations.extend(
            _source_violations(
                ledger,
                ledger_workspace_root,
                package_name=LEDGER_PACKAGE,
                rules=(),
            )
        )
    viewer_violations, viewer, viewer_workspace_root = _viewer_dependency_violations(
        metadata
    )
    violations.extend(viewer_violations)
    if viewer is not None and viewer_workspace_root is not None:
        violations.extend(
            _source_violations(
                viewer,
                viewer_workspace_root,
                package_name=VIEWER_PACKAGE,
                rules=VIEWER_SOURCE_RULES,
            )
        )
    return sorted(set(violations))


def _write(path: pathlib.Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def _write_crate(
    root: pathlib.Path,
    relative_path: str,
    package_name: str,
    *,
    dependencies: Iterable[tuple[str, str]] = (),
    source: str = "pub fn fixture() {}\n",
) -> None:
    dependency_lines = "".join(
        f'{name} = {{ path = "{path}" }}\n' for name, path in dependencies
    )
    crate = root / relative_path
    _write(
        crate / "Cargo.toml",
        f"""[package]
name = "{package_name}"
version = "0.0.0"
edition = "2021"

[dependencies]
{dependency_lines}""",
    )
    _write(crate / "src" / "lib.rs", source)


def _write_workspace(root: pathlib.Path, members: Iterable[str]) -> None:
    quoted_members = ", ".join(json.dumps(member) for member in members)
    _write(
        root / "Cargo.toml",
        f"[workspace]\nmembers = [{quoted_members}]\nresolver = \"2\"\n",
    )


def _write_substrate_fixture(root: pathlib.Path) -> None:
    """Write the exact private active-candidate package used by clean fixtures."""

    declarations = "".join(
        f"mod {module};\n"
        for module in (*SUBSTRATE_ADAPTER_MODULES, *RAW_PLANET_MODULES)
    )
    _write_crate(
        root,
        "crates/planet-substrate",
        SUBSTRATE_PACKAGE,
        dependencies=(
            ("civsim-core", "../core"),
            ("civsim-materials", "../materials"),
            ("civsim-physics", "../physics"),
            ("civsim-units", "../units"),
            ("civsim-world", "../world"),
        ),
        source=declarations,
    )
    for module in RAW_PLANET_MODULES:
        _write(
            root / "crates" / "planet-substrate" / "src" / f"{module}.rs",
            "pub fn retained_fixture() {}\n",
        )
    for module in SUBSTRATE_ADAPTER_MODULES:
        _write(
            root / "crates" / "planet-substrate" / "src" / f"{module}.rs",
            "pub(crate) fn sealed_bridge_fixture() {}\n",
        )


def _fixture_clean(root: pathlib.Path) -> None:
    _write_workspace(
        root,
        (
            "crates/core",
            "crates/ledger",
            "crates/materials",
            "crates/planet",
            "crates/planet-substrate",
            "crates/physics",
            "crates/units",
            "crates/viewer",
            "crates/world",
        ),
    )
    _write_crate(
        root,
        "crates/planet",
        PLANET_PACKAGE,
        dependencies=(
            ("civsim-ledger", "../ledger"),
            ("civsim-units", "../units"),
        ),
        source=(
            "//! The canonical floor-only surface.\n"
            "pub mod canonical;\n"
            "pub struct PlanetSnapshot;\n"
            "pub struct RunReceipt;\n"
            "pub struct PlanetObservation<'a>(&'a RunReceipt);\n"
            "impl<'a> PlanetObservation<'a> {\n"
            "    pub fn receipt(&self) -> &'a RunReceipt { self.0 }\n"
            "}\n"
        ),
    )
    _write_crate(
        root,
        "crates/ledger",
        LEDGER_PACKAGE,
        source=(
            "pub struct Ledger;\n"
            "impl Ledger { pub fn len(&self) -> usize { 0 } }\n"
            "pub struct AbsolutePhysicsFloor;\n"
            "impl AbsolutePhysicsFloor { pub fn receipt(&self) {} }\n"
        ),
    )
    _write(
        root / "crates" / "planet" / "src" / "canonical" / "accounting.rs",
        (
            "//! Protocol prose may name `load_world_ledger`, `WorldProfile`, and\n"
            "//! `PerWorldValue`; only active interfaces are forbidden.\n"
            "use civsim_ledger::{Ledger, Provenance, Tier};\n"
            "pub struct ValidatedAbsolutePhysicsFloor;\n"
            "pub fn run_planet(_floor: &ValidatedAbsolutePhysicsFloor) {}\n"
            "pub fn account(ledger: &Ledger) {\n"
            "    let _ = (ledger.len(), Tier::Reference, Provenance::Authored);\n"
            "}\n"
        ),
    )
    _write_crate(root, "crates/core", "civsim-core")
    _write_crate(root, "crates/materials", "civsim-materials")
    _write_crate(root, "crates/physics", "civsim-physics")
    _write_crate(root, "crates/units", "civsim-units")
    _write_crate(root, "crates/world", "civsim-world")
    _write_substrate_fixture(root)
    _write_crate(
        root,
        "crates/viewer",
        VIEWER_PACKAGE,
        dependencies=(("civsim-planet", "../planet"),),
        source=(
            "use civsim_planet::PlanetObservation;\n"
            "use civsim_planet::PlanetSnapshot;\n"
            "use civsim_planet::RunReceipt;\n"
            "pub struct View<'a> { snapshot: &'a PlanetSnapshot }\n"
            "pub struct RunView<'a> { observation: PlanetObservation<'a> }\n"
            "impl<'a> View<'a> {\n"
            "    pub fn new(snapshot: &'a PlanetSnapshot) -> Self { Self { snapshot } }\n"
            "    pub fn world(&self) -> &'a PlanetSnapshot { self.snapshot }\n"
            "}\n"
            "impl<'a> RunView<'a> {\n"
            "    pub fn new(observation: PlanetObservation<'a>) -> Self { Self { observation } }\n"
            "    pub fn receipt(&self) -> &'a RunReceipt { self.observation.receipt() }\n"
            "}\n"
        ),
    )


def self_test() -> int:
    """Exercise isolated planet and snapshot-observer boundary canaries."""

    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="planet-boundary-gate-") as temporary:
        fixture_root = pathlib.Path(temporary)

        clean_root = fixture_root / "clean"
        _fixture_clean(clean_root)
        clean_violations = check_manifest(clean_root / "Cargo.toml", locked=False)
        if clean_violations:
            failures.append(
                "clean fixture failed, including allowed protocol prose: "
                + "; ".join(clean_violations)
            )

        substrate_surface_root = fixture_root / "substrate-public-surface"
        _fixture_clean(substrate_surface_root)
        _write(
            substrate_surface_root
            / "crates"
            / "planet-substrate"
            / "src"
            / "lib.rs",
            "pub mod astro;\n" + "".join(
                f"mod {module};\n"
                for module in RAW_PLANET_MODULES
                if module != "astro"
            ),
        )
        substrate_surface_violations = check_manifest(
            substrate_surface_root / "Cargo.toml", locked=False
        )
        if not any(
            "`public raw substrate module`" in violation
            for violation in substrate_surface_violations
        ):
            failures.append(
                "public substrate surface fixture was not convicted: "
                + "; ".join(substrate_surface_violations)
            )

        substrate_dependency_root = fixture_root / "canonical-substrate-dependency"
        _fixture_clean(substrate_dependency_root)
        _write_crate(
            substrate_dependency_root,
            "crates/planet",
            PLANET_PACKAGE,
            dependencies=(
                ("civsim-ledger", "../ledger"),
                (SUBSTRATE_PACKAGE, "../planet-substrate"),
                ("civsim-units", "../units"),
            ),
            source="pub mod canonical;\npub struct PlanetSnapshot;\n",
        )
        substrate_dependency_violations = check_manifest(
            substrate_dependency_root / "Cargo.toml", locked=False
        )
        if not any(
            f"forbidden package in planet dependency closure: "
            f"{PLANET_PACKAGE} -> {SUBSTRATE_PACKAGE}" in violation
            for violation in substrate_dependency_violations
        ):
            failures.append(
                "canonical substrate dependency fixture was not convicted: "
                + "; ".join(substrate_dependency_violations)
            )

        ledger_authority_root = fixture_root / "ledger-value-authority"
        _fixture_clean(ledger_authority_root)
        ledger_authority_probe = """pub struct LedgerValue<T> { value: T }
pub struct Ledger;
impl Ledger {
    pub fn bind<T>(&self, value: T) -> LedgerValue<T> { LedgerValue { value } }
}
pub struct AbsolutePhysicsFloor;
impl AbsolutePhysicsFloor {
    pub fn bind<T>(&self, value: T) -> LedgerValue<T> { LedgerValue { value } }
}
"""
        _write(
            ledger_authority_root / "crates" / "ledger" / "src" / "lib.rs",
            ledger_authority_probe,
        )
        ledger_authority_violations = check_manifest(
            ledger_authority_root / "Cargo.toml", locked=False
        )
        missing_ledger_authority_rules = [
            label
            for label, _ in LEDGER_VALUE_AUTHORITY_SOURCE_RULES
            if not any(
                f"`{label}`" in violation
                for violation in ledger_authority_violations
            )
        ]
        if missing_ledger_authority_rules:
            failures.append(
                "ledger value-authority fixture missed: "
                + ", ".join(missing_ledger_authority_rules)
            )

        package_root = fixture_root / "forbidden-package"
        _fixture_clean(package_root)
        _write_workspace(
            package_root,
            (
                "crates/core",
                "crates/ledger",
                "crates/materials",
                "crates/planet",
                "crates/planet-substrate",
                "crates/bridge",
                "crates/physics",
                "crates/units",
                "crates/viewer",
                "crates/world",
                "legacy/sim",
            ),
        )
        _write_crate(
            package_root,
            "crates/planet",
            PLANET_PACKAGE,
            dependencies=(
                ("bridge", "../bridge"),
                ("civsim-ledger", "../ledger"),
                ("civsim-units", "../units"),
            ),
            source="pub mod canonical;\npub struct PlanetSnapshot;\n",
        )
        _write_crate(
            package_root,
            "crates/bridge",
            "bridge",
            dependencies=(("civsim-sim", "../../legacy/sim"),),
        )
        _write_crate(package_root, "legacy/sim", "civsim-sim")
        package_violations = check_manifest(
            package_root / "Cargo.toml", locked=False
        )
        if not any(
            "civsim-planet -> bridge -> civsim-sim" in violation
            for violation in package_violations
        ):
            failures.append(
                "transitive forbidden-package fixture was not convicted: "
                + "; ".join(package_violations)
            )

        parked_root = fixture_root / "parked-path"
        _fixture_clean(parked_root)
        _write_workspace(
            parked_root,
            (
                "crates/core",
                "crates/ledger",
                "crates/materials",
                "crates/planet",
                "crates/planet-substrate",
                "crates/bridge",
                "crates/physics",
                "crates/units",
                "crates/viewer",
                "crates/world",
                "parked/sneaky",
            ),
        )
        _write_crate(
            parked_root,
            "crates/planet",
            PLANET_PACKAGE,
            dependencies=(
                ("bridge", "../bridge"),
                ("civsim-ledger", "../ledger"),
                ("civsim-units", "../units"),
            ),
            source="pub mod canonical;\npub struct PlanetSnapshot;\n",
        )
        _write_crate(
            parked_root,
            "crates/bridge",
            "bridge",
            dependencies=(("sneaky", "../../parked/sneaky"),),
        )
        _write_crate(parked_root, "parked/sneaky", "sneaky")
        parked_violations = check_manifest(parked_root / "Cargo.toml", locked=False)
        if not any("beneath parked/" in violation for violation in parked_violations):
            failures.append(
                "parked-path fixture was not convicted: "
                + "; ".join(parked_violations)
            )

        source_root = fixture_root / "source-tokens"
        _fixture_clean(source_root)
        source_probe = """use civsim_sim::Runner;
use civsim_bio as old_bio;
extern crate civsim_foundation;
fn probe() {
    let _: Option<CalibrationManifest> = None;
    let _: Option<CalibrationError> = None;
    let _ = include_str!("../../../parked/calibration/reserved.toml");
    let _ = Profile :: Calibrated;
    let _ = Profile::Development;
    let _ = dev_default();
    let _ = dev_fixtures();
    let _ = civsim_viewer::render;
    let _ = AbioticSourceRegistry::earth_dev();
    let _ = MIRROR_WORLD_SEED;
    let _ = Environment::local_disk_solar_pin();
}
"""
        _write(
            source_root / "crates" / "planet" / "src" / "probe.rs",
            source_probe,
        )
        source_violations = check_manifest(source_root / "Cargo.toml", locked=False)
        missing_rules = [
            label
            for label, _ in PLANET_SOURCE_RULES
            if not any(f"`{label}`" in violation for violation in source_violations)
        ]
        if missing_rules:
            failures.append(
                "source-token fixture missed: " + ", ".join(missing_rules)
            )

        front_door_root = fixture_root / "front-door-ingress"
        _fixture_clean(front_door_root)
        front_door_probe = """use civsim_ledger::Ledger;
use civsim_physics::hayashi_wall::HayashiWallGrid;
use crate::astro::derive_planet;
pub fn calibrate_stage() {}
fn run_planet(spec: &PlanetRunSpec, raw: &Ledger) {}
fn probe(world_ledger: &Ledger, floor: &AbsolutePhysicsFloor) {
    let _: f64 = 0.0;
    let _ = load_world_ledger("world.toml");
    let _ = WorldProfile::from_path("profile.toml");
    let _ = WorldManifest::load("manifest.toml");
    let _ = world_ledger.bind("world.mass", Fixed::ONE);
    let _ = floor.bind("fundamental.G", Fixed::from_num(999));
    let _ = floor.lookup("fundamental.G");
    let _: Option<AuthoredWorldValue> = None;
    let _: Option<PerWorldValue> = None;
    let _ = ValidatedAbsolutePhysicsFloor::new_unchecked();
}
pub enum PlanetRunOutcome { Refused }
pub struct PlanetRunOutcome { pub receipt: u8 }
pub struct PlanetRunOutcome(pub u8);
pub struct PlanetRunOutcome;
impl PlanetRunOutcome { pub fn forge() -> Self { loop {} } }
"""
        _write(
            front_door_root
            / "crates"
            / "planet"
            / "src"
            / "canonical"
            / "stages"
            / "stellar_birth"
            / "front_door_probe.rs",
            front_door_probe,
        )
        front_door_violations = check_manifest(
            front_door_root / "Cargo.toml", locked=False
        )
        missing_front_door_rules = [
            label
            for label, _ in PLANET_FRONT_DOOR_SOURCE_RULES
            if not any(
                f"`{label}`" in violation
                for violation in front_door_violations
            )
        ]
        if missing_front_door_rules:
            failures.append(
                "nested front-door source-token fixture missed: "
                + ", ".join(missing_front_door_rules)
            )

        public_surface_root = fixture_root / "public-surface"
        _fixture_clean(public_surface_root)
        _write(
            public_surface_root / "crates" / "planet" / "src" / "lib.rs",
            "pub mod canonical;\npub mod astro;\npub use planet::derive_planet;\n",
        )
        public_surface_violations = check_manifest(
            public_surface_root / "Cargo.toml", locked=False
        )
        missing_public_surface_rules = [
            label
            for label, _ in PLANET_PUBLIC_SURFACE_SOURCE_RULES
            if not any(
                f"`{label}`" in violation
                for violation in public_surface_violations
            )
        ]
        if missing_public_surface_rules:
            failures.append(
                "public planet surface fixture missed: "
                + ", ".join(missing_public_surface_rules)
            )

        viewer_dependency_root = fixture_root / "viewer-forbidden-dependency"
        _fixture_clean(viewer_dependency_root)
        existing_forbidden = {
            "civsim-materials": "../materials",
            "civsim-physics": "../physics",
            SUBSTRATE_PACKAGE: "../planet-substrate",
            "civsim-units": "../units",
            "civsim-world": "../world",
        }
        forbidden_members = tuple(
            f"forbidden/{name}"
            for name in sorted(VIEWER_FORBIDDEN_PACKAGES)
            if name not in existing_forbidden
        )
        _write_workspace(
            viewer_dependency_root,
            (
                "crates/core",
                "crates/ledger",
                "crates/materials",
                "crates/planet",
                "crates/planet-substrate",
                "crates/physics",
                "crates/units",
                "crates/viewer",
                "crates/viewer-helper",
                "crates/world",
            )
            + forbidden_members,
        )
        _write_crate(
            viewer_dependency_root,
            "crates/viewer",
            VIEWER_PACKAGE,
            dependencies=(
                ("civsim-planet", "../planet"),
                ("viewer-helper", "../viewer-helper"),
            ),
            source=(
                "use civsim_planet::PlanetSnapshot;\n"
                "pub fn observe(snapshot: &PlanetSnapshot) -> &PlanetSnapshot { snapshot }\n"
            ),
        )
        _write_crate(
            viewer_dependency_root,
            "crates/viewer-helper",
            "viewer-helper",
            dependencies=tuple(
                (
                    name,
                    existing_forbidden.get(name, f"../../forbidden/{name}"),
                )
                for name in sorted(VIEWER_FORBIDDEN_PACKAGES)
            ),
        )
        for forbidden_name in sorted(VIEWER_FORBIDDEN_PACKAGES):
            if forbidden_name not in existing_forbidden:
                _write_crate(
                    viewer_dependency_root,
                    f"forbidden/{forbidden_name}",
                    forbidden_name,
                )
        viewer_dependency_violations = check_manifest(
            viewer_dependency_root / "Cargo.toml", locked=False
        )
        missing_forbidden_dependencies = [
            name
            for name in sorted(VIEWER_FORBIDDEN_PACKAGES)
            if not any(
                f"civsim-viewer -> viewer-helper -> {name}" in violation
                for violation in viewer_dependency_violations
            )
        ]
        if missing_forbidden_dependencies:
            failures.append(
                "transitive viewer dependency fixture missed: "
                + ", ".join(missing_forbidden_dependencies)
                + "; violations: "
                + "; ".join(viewer_dependency_violations)
            )

        viewer_legacy_root = fixture_root / "viewer-parked-legacy"
        _fixture_clean(viewer_legacy_root)
        _write_workspace(
            viewer_legacy_root,
            (
                "crates/core",
                "crates/ledger",
                "crates/materials",
                "crates/planet",
                "crates/planet-substrate",
                "crates/physics",
                "crates/units",
                "crates/viewer",
                "crates/world",
                "parked/renderer",
            ),
        )
        _write_crate(
            viewer_legacy_root,
            "crates/viewer",
            VIEWER_PACKAGE,
            dependencies=(
                ("civsim-planet", "../planet"),
                ("civsim-render-legacy", "../../parked/renderer"),
            ),
            source=(
                "use civsim_planet::PlanetSnapshot;\n"
                "pub fn observe(snapshot: &PlanetSnapshot) -> &PlanetSnapshot { snapshot }\n"
            ),
        )
        _write_crate(
            viewer_legacy_root,
            "parked/renderer",
            "civsim-render-legacy",
        )
        viewer_legacy_violations = check_manifest(
            viewer_legacy_root / "Cargo.toml", locked=False
        )
        if not any(
            "parked/legacy package in viewer dependency closure" in violation
            for violation in viewer_legacy_violations
        ) or not any(
            "legacy package in viewer dependency closure" in violation
            for violation in viewer_legacy_violations
        ):
            failures.append(
                "parked legacy viewer dependency fixture was not convicted by both boundaries: "
                + "; ".join(viewer_legacy_violations)
            )

        viewer_source_root = fixture_root / "viewer-source-tokens"
        _fixture_clean(viewer_source_root)
        viewer_source_probe = """use civsim_bio::Genome;
use civsim_compose::Node;
use civsim_foundation::Clock;
use civsim_gpu::Device;
use civsim_materials::Material;
use civsim_physics::PhysicsRegistry;
use civsim_sim::Runner;
use civsim_units::Quantity;
use civsim_world::Coord3;
extern crate civsim_viewer_legacy;
use civsim_planet as causal_planet;
use civsim_planet::run_planet;
use civsim_planet::{PlanetObservation, PlanetSnapshot, RunReceipt};
use civsim_planet::PlanetSnapshot as HiddenSnapshot;

type HiddenObservation<'a> = PlanetObservation<'a>;

const MIRROR_WORLD_SEED: u64 = 0;
const SUN_DEFAULT: u8 = 1;
const FIXED_ORBIT_AU: u8 = 1;
const PHYSICAL_TIMESTEP_MYR: u8 = 20;

fn own(snapshot: PlanetSnapshot) -> PlanetSnapshot { snapshot }
fn mutate(snapshot: &mut PlanetSnapshot) {
    snapshot.advance();
}
fn mutate_observation(observation: &mut PlanetObservation<'_>) {
    observation.advance();
}
fn construct_observation() {
    let _ = PlanetObservation::default();
}
fn mutate_receipt(receipt: &mut RunReceipt) {
    receipt.repair();
}
fn own_receipt(receipt: RunReceipt) -> RunReceipt { receipt }
fn construct_receipt() {
    let _ = RunReceipt::default();
}

fn probe() {
    let _ = include_str!("../../../parked/viewer/input.toml");
    let _ = PlanetSnapshot::default();
    let _ = PlanetRunSpec;
    let _ = PlanetRunOutcome;
    preflight();
    readiness_receipt();
    audited_substrate_ledger();
    build_globe_fixture();
    let _ = DerivedScene;
    derive_pre_ms_bolometric_luminosity();
    step_provinces();
    let _ = AbioticSourceRegistry::earth_dev();
    let _ = Environment::local_disk_solar_pin();
    let _ = None::<&str>.unwrap_or("earth");
    let orbit = OrbitInput { orbit_au: Fixed::ONE };
    let composition = vec![("O", 1)];
    let fallback = None::<Fixed>.unwrap_or(Fixed::ONE);
}
"""
        _write(
            viewer_source_root / "crates" / "viewer" / "src" / "probe.rs",
            viewer_source_probe,
        )
        viewer_source_violations = check_manifest(
            viewer_source_root / "Cargo.toml", locked=False
        )
        missing_viewer_rules = [
            label
            for label, _ in VIEWER_SOURCE_RULES
            if not any(
                f"`{label}`" in violation
                for violation in viewer_source_violations
            )
        ]
        if missing_viewer_rules:
            failures.append(
                "viewer source-token fixture missed: "
                + ", ".join(missing_viewer_rules)
            )

    if failures:
        print("planet boundary gate self-test: FAIL")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print(
        "planet boundary gate self-test: PASS "
        "(ledger value authority, private active-candidate substrate, typed-adapter-only canonical "
        "front door, integer-only canonical arithmetic, generic floor lookup rejection, viewer "
        "dependency, immutable-observation source, and alien-default canaries exercised)"
    )
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run deterministic canaries in isolated temporary workspaces",
    )
    parser.add_argument(
        "--manifest-path",
        type=pathlib.Path,
        default=ROOT / "Cargo.toml",
        help="workspace manifest to inspect (default: repository Cargo.toml)",
    )
    arguments = parser.parse_args(argv)
    if arguments.self_test:
        return self_test()

    violations = check_manifest(arguments.manifest_path)
    if violations:
        print("planet boundary gate: FAIL")
        for violation in violations:
            print(f"  - {violation}")
        return 1
    print(
        "planet boundary gate: clean "
        "(ledger cannot authorize values, fourteen raw planet modules and the sealed floor bridge are "
        "private in the separate active-candidate package, canonical runtime reaches only ledger and "
        "units until typed adapters land, canonical arithmetic is integer-only, and viewer is a "
        "an immutable-observation leaf)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
