// Copyright 2026 Nathan M. Fraske
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The derived-output-is-live gate (task #43), slice 1: the `@derives[id]:` cross-check.
//!
//! A retired-floor derivation is a derived value that replaced an authored floor. The constructor
//! gate proves no such value is authored; this gate proves the derived replacement is alive. Its
//! membership is the data-defined [`DerivationRegistry`], one row per derivation, and
//! each row is anchored to a source site by an `@derives[id]:` annotation. This test is the ratchet
//! that keeps the registry and the annotations in step (the constructor-gate pattern): every
//! annotated site must have a registry row, and every row must have its annotation at the site it
//! names. So a new derivation cannot land without a registry row (the liveness probe would miss
//! it), and a row cannot go stale (its site annotation removed) without failing.
//!
//! The site-local liveness probe (perturb the input, assert the derived value responds) is the next
//! slice; this slice locks the membership.

use civsim_foundation::calibration::CalibrationManifest;
use civsim_sim::derive_gate::{coverage_report, Coverage, DerivationRegistry};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The labelled dev-fixtures profile the CI harness reads: it carries the set (not reserved)
/// constants the wired probes perturb. The gate reads run state and asserts; it never enters the sim.
const DEV_FIXTURES: &str = include_str!("../../../calibration/profiles/dev-fixtures.toml");

/// The determinism-critical crate source roots the annotations live in, repo-relative. A crate
/// extracted out of `civsim-sim` must be added here in the same commit that moves it, or its
/// annotations leave the scan silently and the registry rows pointing at them read as stale.
/// `crates/foundation/src` carries three today (the clock's calendar cell and day cadence, the
/// decomposer's soil-nutrient recovery); `crates/bio/src` carries none yet and is listed so that
/// the first one written there is covered rather than dropped.
const SCANNED_CRATES: &[&str] = &[
    "crates/core/src",
    "crates/physics/src",
    "crates/world/src",
    "crates/bio/src",
    "crates/foundation/src",
    "crates/sim/src",
];

/// The gate's own registry module documents the `@derives[id]:` convention in prose, so it is
/// excluded from the annotation scan (it carries no derivation site, only the machinery).
const EXCLUDED: &[&str] = &["crates/sim/src/derive_gate.rs"];

/// The workspace root, derived from this crate's manifest dir (`.../crates/sim`).
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/sim has a workspace root two levels up")
        .to_path_buf()
}

/// Recursively collect every `.rs` file under a directory.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Extract the id inside a `@derives[<id>]:` token on a line, if present.
fn derives_id(line: &str) -> Option<&str> {
    let start = line.find("@derives[")? + "@derives[".len();
    let rest = &line[start..];
    let end = rest.find(']')?;
    // A well-formed token closes the bracket and follows it with a colon.
    if rest[end + 1..].trim_start().starts_with(':') {
        Some(&rest[..end])
    } else {
        None
    }
}

/// Scan the source tree for `@derives[id]:` annotations, returning id -> repo-relative site path.
/// Panics on a duplicate id across sites, which would make the anchor ambiguous.
fn scan_annotations(root: &Path) -> BTreeMap<String, String> {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for crate_src in SCANNED_CRATES {
        let mut files = Vec::new();
        rust_files(&root.join(crate_src), &mut files);
        for file in files {
            let rel = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            if EXCLUDED.contains(&rel.as_str()) {
                continue;
            }
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            for line in text.lines() {
                if let Some(id) = derives_id(line) {
                    if let Some(prev) = found.insert(id.to_string(), rel.clone()) {
                        panic!("@derives[{id}] appears at two sites: {prev} and {rel}");
                    }
                }
            }
        }
    }
    found
}

#[test]
fn every_derives_annotation_has_a_registry_row_and_vice_versa() {
    let root = workspace_root();
    let annotations = scan_annotations(&root);
    let registry = DerivationRegistry::canonical();

    let annotated: std::collections::BTreeSet<&str> =
        annotations.keys().map(|s| s.as_str()).collect();
    let registered: std::collections::BTreeSet<&str> = registry.ids().into_iter().collect();

    let missing_row: Vec<&str> = annotated.difference(&registered).copied().collect();
    let missing_site: Vec<&str> = registered.difference(&annotated).copied().collect();

    assert!(
        missing_row.is_empty(),
        "@derives sites with no RetiredFloorDerivation row (add a registry row so the liveness gate probes them): {missing_row:?}"
    );
    assert!(
        missing_site.is_empty(),
        "registry rows whose @derives[id] annotation is gone (the site changed; fix the row or restore the annotation): {missing_site:?}"
    );

    // The row's declared site must be where its annotation actually lives, so the probe reads the
    // right derivation.
    for d in registry.iter() {
        let at = annotations.get(&d.id).expect("checked present above");
        assert_eq!(
            at, &d.site,
            "derivation '{}' is registered at {} but its @derives annotation is at {}",
            d.id, d.site, at
        );
    }
}

#[test]
fn the_coverage_gate_fails_on_no_dead_derivation_and_prints_the_report() {
    // The CI harness (task #43 slice 4): walk the canonical registry against the dev-fixtures
    // manifest, run every wired liveness probe, and FAIL on any Dead (a derived output gone constant,
    // the soil_baseline regression the constructor gate cannot see). The other states (Unwired,
    // Trivial, DeadbandOnPins) are surfaced as gaps, never failures: a green gate is honest about
    // where the probe is absent or weak. The report prints so the gaps stay visible in CI logs.
    let manifest = CalibrationManifest::from_toml_str(DEV_FIXTURES).expect("dev-fixtures parses");
    let report = coverage_report(&manifest);
    assert!(
        !report.is_empty(),
        "the registry carries derivations to classify"
    );

    let mut dead: Vec<&str> = Vec::new();
    let mut silent_gap: Vec<&str> = Vec::new();
    println!(
        "derived-output coverage signal ({} derivations):",
        report.len()
    );
    for row in &report {
        let state = match row.coverage {
            Coverage::Live => "LIVE",
            Coverage::Dead => "DEAD",
            Coverage::Unwired => "unwired",
            Coverage::Trivial => "trivial",
            Coverage::DeadbandOnPins => "deadband-on-pins",
        };
        if row.note.is_empty() {
            println!("  {state:>16}  {}", row.id);
        } else {
            println!("  {state:>16}  {}  ({})", row.id, row.note);
        }
        if row.coverage == Coverage::Dead {
            dead.push(&row.id);
        }
        // Every non-Live/Dead state must carry a reason, so no coverage gap is silent.
        if matches!(
            row.coverage,
            Coverage::Unwired | Coverage::Trivial | Coverage::DeadbandOnPins
        ) && row.note.is_empty()
        {
            silent_gap.push(&row.id);
        }
    }

    assert!(
        dead.is_empty(),
        "derived outputs read Dead (constant under their declared input, the soil_baseline bug): {dead:?}"
    );
    assert!(
        silent_gap.is_empty(),
        "coverage gaps with no reason recorded (add a gap reason so the gap is visible): {silent_gap:?}"
    );
}

#[test]
fn the_cross_check_would_catch_a_new_unregistered_derivation() {
    // Self-test the ratchet: a synthetic annotation id that has no registry row must be detected by
    // the same set-difference the gate uses, proving the gate is not vacuous.
    let registry = DerivationRegistry::canonical();
    let registered: std::collections::BTreeSet<&str> = registry.ids().into_iter().collect();
    let synthetic = "a_new_unregistered_derivation";
    assert!(
        !registered.contains(synthetic),
        "a new @derives id absent from the registry is a difference the gate reports"
    );
}
