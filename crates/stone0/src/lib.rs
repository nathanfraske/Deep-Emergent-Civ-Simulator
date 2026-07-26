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

//! # civsim-stone0: the local-firing provenance gate
//!
//! Stone 0 is the meta-gate that makes the no-fabricated-values discipline un-bypassable at the local
//! inner loop (design `docs/working/Q1_STONE0_PROVENANCE_GATE_DESIGN.md`). This crate is the gate's
//! library and its `stone0-gate` binary. INCREMENT 4 IS DONE and the gate fires at BUILD time.
//! `crates/stone0-build` is the single shared build-graph owner for the canonical planet and
//! planet-substrate consumers. It calls `run_at_repository_root(Mode::Local, root)` with the root
//! derived from its own manifest directory and panics on a positive detection, so one scan guards each
//! canonical Cargo graph instead of one scan per consumer. The parked compatibility runner retains the
//! same guard in `parked/crates/sim/build.rs`. The binary remains the direct entry point for `--ci` and
//! `--self-test`.
//!
//! Said plainly because the previous wording claimed the opposite ("NOT yet wired into any build script"),
//! which would send a developer whose build is blocked here to rule this gate out as the cause when it is
//! the most likely one.
//!
//! ## The checks
//!
//! 1. Provenance scan (the violation predicate): invokes the declarative gate runner's canonical tier,
//!    including every per-source receipt test, and collects any failure. An owner override may admit
//!    only detections reported by that ordinary runner. Stone 0 also executes its mandatory authority
//!    commands directly; their detections and operational failures are always hard failures.
//!    Cargo decides when a guarded build script must rerun. Direct Stone 0 calls always rescan, so a
//!    verdict can never outlive an input that a gate reads outside the repository.
//! 2. Live-password laundering scan (local only): if the secrets file exists, reads the password and
//!    scans every tracked or nonignored untracked worktree file plus the git index for the literal
//!    password and its base64. A hit is a hard fail. The password value is never printed, logged,
//!    written, or persisted; only the file where a hit occurred is reported.
//! 3. Tombstone scan: every retired (now-declassified) override phrase in `stone0_tombstones.txt` must
//!    appear nowhere but the tombstone list itself. A hit is a laundered stale copy and fails.
//! 4. Override-env-name scan: a committed file that assigns `STONE0_OVERRIDE` has baked the override in
//!    and fails.
//! 5. Canary: a `CANARY=` line in the secrets file that appears anywhere in the tracked tree proves the
//!    secrets file was read and its contents committed, and fails.
//!
//! ## Robustness contract
//!
//! The authoritative provenance runner fails closed when its root-owned interpreter is unavailable,
//! cannot enter isolated mode, or does not return the invocation-bound wrapper-completion receipt.
//! Python import-control environment is removed before every authority subprocess. Native Windows
//! execution remains closed until it has an independently verifiable interpreter trust root; the
//! supported Windows development route executes Stone 0 inside WSL. A missing secrets file still
//! disables only the optional owner override and live-secret canary checks, while native
//! repository-service errors remain visible warnings. A gate that cannot execute the canonical
//! provenance tier has not passed.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// The default out-of-repo secrets file the owner keeps the override password in.
pub const DEFAULT_SECRETS_PATH: &str = "/mnt/e/Secrets/stone0-override.pass";
/// The committed tombstone list, one retired (declassified) override phrase per line.
pub const TOMBSTONE_REL: &str = "scripts/stone0_tombstones.txt";
/// The environment variable an owner-authorized single-command override is supplied through.
pub const OVERRIDE_ENV: &str = "STONE0_OVERRIDE";

/// Emit Cargo rebuild inputs for a build script that invokes Stone 0.
///
/// Existing tracked files are listed one by one, and the Git index is watched
/// so adding a tracked file refreshes that list. A small directory fallback
/// keeps the guard live when Git is unavailable. External vendored custody and
/// the secrets path are watched separately because the Python receipt tests may
/// read them even though they cannot appear in `git ls-files`.
pub fn emit_cargo_rerun_inputs(repo_root: &Path) {
    for relative in [
        "crates",
        "sources",
        "scripts",
        "docs/SOURCES.md",
        "docs/working/PHYSICS_FLOOR_REGISTRY.md",
        "docs/working/CANONICAL_LEDGER_INVENTORY.txt",
        "Cargo.toml",
        "Cargo.lock",
    ] {
        println!(
            "cargo:rerun-if-changed={}",
            repo_root.join(relative).display()
        );
    }

    if let Ok(mut command) = trusted_git_command(repo_root) {
        if let Ok(output) = command.args(["ls-files", "-z"]).output() {
            if output.status.success() {
                for raw in output.stdout.split(|byte| *byte == 0) {
                    if raw.is_empty() || raw.contains(&b'\n') || raw.contains(&b'\r') {
                        continue;
                    }
                    let relative = String::from_utf8_lossy(raw);
                    println!(
                        "cargo:rerun-if-changed={}",
                        repo_root.join(relative.as_ref()).display()
                    );
                }
            }
        }
    }

    if let Ok(mut command) = trusted_git_command(repo_root) {
        if let Ok(output) = command
            .args(["rev-parse", "--path-format=absolute", "--git-path", "index"])
            .output()
        {
            if output.status.success() {
                let index = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !index.is_empty() {
                    println!("cargo:rerun-if-changed={index}");
                }
            }
        }
    }

    if let Some(home) = std::env::var_os("HOME") {
        let custody = PathBuf::from(home).join(".claude/vendored-sources");
        if custody.exists() {
            println!("cargo:rerun-if-changed={}", custody.display());
        } else if let Some(parent) = custody.parent().filter(|path| path.exists()) {
            println!("cargo:rerun-if-changed={}", parent.display());
        }
    }
    let secrets = std::env::var_os("STONE0_SECRETS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SECRETS_PATH));
    if secrets.exists() {
        println!("cargo:rerun-if-changed={}", secrets.display());
    } else if let Some(parent) = secrets.parent().filter(|path| path.exists()) {
        println!("cargo:rerun-if-changed={}", parent.display());
    }
    println!("cargo:rerun-if-env-changed={OVERRIDE_ENV}");
    println!("cargo:rerun-if-env-changed=STONE0_SECRETS_PATH");
}

/// The instruction block printed when the provenance scan fails with no valid override.
pub const OVERRIDE_INSTRUCTIONS: &str = "This value has no provenance. To override you must obtain the current password from Nathan (out of band), set STONE0_OVERRIDE for a single command, and Nathan will rotate the password afterward. Do NOT write the password into the repo; CI will catch it via the tombstone list and halt.";

/// The bootstrap pointer into the declarative gate authority.
///
/// Gate membership, order, arguments, timeouts, inputs, path triggers, cache policy, and self-test
/// metadata live in `scripts/gates.toml`. Stone 0 owns this runner pointer and an exact bootstrap pin
/// for the authority-watchdog block. The narrow duplication prevents deleting or weakening the gate
/// that validates the inventory from also deleting its own execution.
const PROVENANCE_RUNNER: (&str, &[&str]) = (
    "scripts/gate_runner.py",
    &["run", "--tier", "canonical", "--phase", "provenance"],
);
const MANDATORY_AUTHORITY_COMMANDS: [(&str, &[&str]); 5] = [
    ("scripts/authority_watchdog_gate.py", &[]),
    ("scripts/codata_floor_evidence_gate.py", &[]),
    ("scripts/stone0_build_wiring_gate.py", &[]),
    ("scripts/fixed_math_authority_gate.py", &[]),
    ("scripts/external_claim_gate.py", &[]),
];
const GATE_MANIFEST_PATH: &str = "scripts/gates.toml";
const MANDATORY_AUTHORITY_GATE_BLOCK: &str = r#"[[gate]]
id = "canonical.authority-watchdog"
order = 65
description = "Require independent pairs for active authority-bearing mechanical claims."
tiers = ["canonical", "doctor", "pr", "full", "nightly", "stop"]
phase = "provenance"
command = ["{python}", "scripts/authority_watchdog_gate.py"]
self_test = ["{python}", "scripts/authority_watchdog_gate.py", "--self-test"]
timeout_seconds = 120
cache = "content-hash"
inputs = [
  "scripts/gates.toml",
  "scripts/gate_runner.py",
  "scripts/authority_watchdog_gate.py",
  "scripts/authority_registry_watchdog.py",
  "scripts/authority_watchdog.toml",
  "scripts/codata_floor_evidence_gate.py",
  "scripts/codata_floor_evidence_watchdog.py",
  "scripts/external_claim_gate.py",
  "scripts/external_claim_watchdog.py",
  "scripts/fixed_math_authority_gate.py",
  "scripts/fixed_math_authority_watchdog.py",
  "scripts/stone0_build_wiring_gate.py",
  "scripts/stone0_build_wiring_watchdog.py",
  "sources/external_claims.toml",
  "sources/external_claim_approvers",
  "sources/external_claim_revocations.toml",
  "sources/registry.toml",
  "docs/working/INDEPENDENT_AUTHORITY_RULE.md",
  "docs/working/EXTERNAL_ADVERSE_CLAIM_RULE.md",
  "crates",
]
path_triggers = [
  "scripts/gates.toml",
  "scripts/gate_runner.py",
  "scripts/authority_watchdog_gate.py",
  "scripts/authority_registry_watchdog.py",
  "scripts/authority_watchdog.toml",
  "scripts/codata_floor_evidence_gate.py",
  "scripts/codata_floor_evidence_watchdog.py",
  "scripts/external_claim_gate.py",
  "scripts/external_claim_watchdog.py",
  "scripts/fixed_math_authority_gate.py",
  "scripts/fixed_math_authority_watchdog.py",
  "scripts/stone0_build_wiring_gate.py",
  "scripts/stone0_build_wiring_watchdog.py",
  "sources/external_claims.toml",
  "sources/external_claim_approvers",
  "sources/external_claim_revocations.toml",
  "sources/registry.toml",
  "docs/working/INDEPENDENT_AUTHORITY_RULE.md",
  "docs/working/EXTERNAL_ADVERSE_CLAIM_RULE.md",
  "crates/**",
]"#;
const POLICY_DETECTION_MARKER: &str = "civsim.gate-runner.policy-detection.v1";

/// Which run this is. `Local` runs every check including the live-password and canary scans against the
/// secrets file. `Ci` runs the provenance, tombstone, and override-env scans only (a hosted runner has
/// no secrets dir and must never attempt the live-password scan). `SelfTest` runs the internal proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Local,
    Ci,
    SelfTest,
}

/// The gate's verdict. `Clean` and `Overridden` both allow the build (exit 0); `Fail` blocks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Clean,
    Overridden,
    Fail,
}

/// The full result of a gate run.
#[derive(Debug, Clone)]
pub struct GateReport {
    pub verdict: Verdict,
    /// Positive detections and mandatory authority failures that block the build.
    pub failures: Vec<String>,
    /// Non-authoritative operational notices. Canonical provenance-runner
    /// unavailability is promoted to a failure before this report is returned.
    pub warnings: Vec<String>,
    /// Informational notices (the loud override banner text lives here).
    pub notices: Vec<String>,
}

impl GateReport {
    /// The process exit code: 1 on a hard fail, 0 otherwise.
    pub fn exit_code(&self) -> i32 {
        if self.verdict == Verdict::Fail {
            1
        } else {
            0
        }
    }
}

/// The inputs a gate run keys off. Constructed from the environment by [`run`], or by hand in tests.
pub struct GateConfig {
    repo_root: PathBuf,
    secrets_path: PathBuf,
    tombstones_path: PathBuf,
    mode: Mode,
    /// The value supplied through `STONE0_OVERRIDE`, if any (only consulted in `Local` mode).
    override_value: Option<String>,
}

impl GateConfig {
    /// Build a scan-only configuration with no authority to override a finding.
    ///
    /// This is the public construction path used by tests and embedding tools.
    /// Only [`run`] may attach the environment's owner override after validating
    /// that the default out-of-repo trust path is in use.
    pub fn scan_only(
        repo_root: PathBuf,
        secrets_path: PathBuf,
        tombstones_path: PathBuf,
        mode: Mode,
    ) -> Self {
        Self {
            repo_root,
            secrets_path,
            tombstones_path,
            mode,
            override_value: None,
        }
    }
}

/// One tracked file's path and raw bytes. The password scan is byte-exact (it catches a hit even in a
/// binary blob); the text scans read the bytes as UTF-8 lossily.
#[derive(Debug, Clone)]
pub struct RepoFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

impl RepoFile {
    /// Build a file record from any byte content (used by tests).
    pub fn new(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        RepoFile {
            path: path.into(),
            bytes: bytes.into(),
        }
    }

    fn text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes)
    }
}

// ----------------------------------------------------------------------------------------------------
// Inline base64 (standard alphabet, with padding). Implemented here so the gate depends on nothing.
// ----------------------------------------------------------------------------------------------------

/// Standard base64 (RFC 4648 alphabet, `=` padded) of the input bytes.
pub fn base64_standard(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHA[((n >> 18) & 63) as usize] as char);
        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHA[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHA[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

// ----------------------------------------------------------------------------------------------------
// Pure detection functions. Each takes explicit inputs so it is testable without touching the real tree.
// ----------------------------------------------------------------------------------------------------

/// True if `needle` occurs as a contiguous subslice of `hay`. An empty needle never matches.
fn contains_bytes(hay: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > hay.len() {
        return false;
    }
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Parse the secrets file content into `(password, canary)`. The password is the first non-empty line
/// that does not start with `CANARY=`, trimmed. The canary, if present, is the value after `CANARY=`.
pub fn parse_secrets(content: &str) -> (String, Option<String>) {
    let mut password = String::new();
    let mut canary: Option<String> = None;
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix("CANARY=") {
            if canary.is_none() && !rest.is_empty() {
                canary = Some(rest.to_string());
            }
            continue;
        }
        if password.is_empty() {
            password = t.to_string();
        }
    }
    (password, canary)
}

/// Parse the tombstone list: one retired phrase per non-empty, non-comment line, trimmed.
pub fn parse_tombstones(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// The files (by path) that contain the live password, its padded base64, or its unpadded base64.
/// An empty password matches nothing. The password value itself is never included in the output.
pub fn find_password_hits(files: &[RepoFile], password: &str) -> Vec<String> {
    if password.is_empty() {
        return Vec::new();
    }
    let lit = password.as_bytes();
    let b64 = base64_standard(password.as_bytes());
    let b64_nopad = b64.trim_end_matches('=').to_string();
    let mut hits = Vec::new();
    for f in files {
        let hit = contains_bytes(&f.bytes, lit)
            || contains_bytes(&f.bytes, b64.as_bytes())
            || (!b64_nopad.is_empty() && contains_bytes(&f.bytes, b64_nopad.as_bytes()));
        if hit {
            hits.push(f.path.clone());
        }
    }
    hits
}

/// The `(path, phrase)` hits where a retired tombstone phrase appears in a file other than the excluded
/// tombstone list itself. The phrase is safe to report: retirement declassifies it.
pub fn find_tombstone_hits(
    files: &[RepoFile],
    tombstones: &[String],
    exclude_exact: &[String],
) -> Vec<(String, String)> {
    let mut hits = Vec::new();
    for f in files {
        if exclude_exact.contains(&f.path) {
            continue;
        }
        let text = f.text();
        for t in tombstones {
            if t.is_empty() {
                continue;
            }
            if text.contains(t.as_str()) {
                hits.push((f.path.clone(), t.clone()));
            }
        }
    }
    hits
}

/// The files that assign the `STONE0_OVERRIDE` environment variable (the name followed, after optional
/// spaces or tabs, by `=` or `:`), excluding any file whose path starts with an excluded prefix.
pub fn find_override_env_hits(files: &[RepoFile], exclude_prefixes: &[String]) -> Vec<String> {
    let mut hits = Vec::new();
    'file: for f in files {
        if exclude_prefixes
            .iter()
            .any(|p| f.path.starts_with(p.as_str()))
        {
            continue;
        }
        let text = f.text();
        for (idx, _) in text.match_indices(OVERRIDE_ENV) {
            let after = &text[idx + OVERRIDE_ENV.len()..];
            let trimmed = after.trim_start_matches([' ', '\t']);
            if trimmed.starts_with('=') || trimmed.starts_with(':') {
                hits.push(f.path.clone());
                continue 'file;
            }
        }
    }
    hits
}

/// The files that contain the secrets-file canary value (excluding any exact-path exclusion). A hit
/// proves the secrets file was read directly and its content committed.
pub fn find_canary_hits(files: &[RepoFile], canary: &str, exclude_exact: &[String]) -> Vec<String> {
    if canary.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for f in files {
        if exclude_exact.contains(&f.path) {
            continue;
        }
        if contains_bytes(&f.bytes, canary.as_bytes()) {
            hits.push(f.path.clone());
        }
    }
    hits
}

/// True if a non-empty override value equals the secrets-file password. Never logs either value.
pub fn override_is_valid(override_value: Option<&str>, password: &str) -> bool {
    match override_value {
        Some(v) => !password.is_empty() && !v.is_empty() && v == password,
        None => false,
    }
}

// ----------------------------------------------------------------------------------------------------
// The provenance scan: shell out to every authoritative Python gate.
// ----------------------------------------------------------------------------------------------------

/// What one gate run produced.
///
/// `Skipped` USED TO EXIST and is deliberately gone. A missing runner, a crashing
/// runner, and an unavailable interpreter are distinct diagnostics but share
/// one non-overridable result: the authority did not execute, so the build fails.
enum ScriptResult {
    Clean,
    Detected(String),
    Operational(String),
}

struct ProvenanceOutcome {
    /// Policy detections from the ordinary declarative provenance runner.
    ///
    /// This is the sole bucket that the owner-password path may override.
    override_eligible_detections: Vec<String>,
    /// Failures whose authority semantics prohibit an owner override.
    ///
    /// This includes every operational failure plus both semantic detections
    /// and operational failures from the direct mandatory authority commands.
    non_overridable_failures: Vec<String>,
}

const PYTHON_SUCCESS_RECEIPT_SCHEMA: &str = "civsim.stone0.python-authority-success.v1";
const PYTHON_WRAPPER: &str = r#"
import runpy
import sys

if not (
    sys.flags.isolated
    and sys.flags.ignore_environment
    and sys.flags.no_user_site
    and sys.flags.no_site
    and sys.flags.dont_write_bytecode
):
    raise SystemExit("Stone 0 requires isolated Python execution")

receipt = sys.argv[1]
script = sys.argv[2]
sys.argv = [script, *sys.argv[3:]]
try:
    runpy.run_path(script, run_name="__main__")
except SystemExit as error:
    code = error.code
    if code is None:
        code = 0
    if not isinstance(code, int) or code != 0:
        raise
print(receipt, file=sys.stderr)
"#;
const SCRUBBED_PYTHON_ENVIRONMENT: [&str; 16] = [
    "PYTHONHOME",
    "PYTHONPATH",
    "PYTHONSTARTUP",
    "PYTHONINSPECT",
    "PYTHONUSERBASE",
    "PYTHONWARNINGS",
    "PYTHONBREAKPOINT",
    "PYTHONEXECUTABLE",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    "LD_DEBUG",
    "LD_DEBUG_OUTPUT",
    "LD_PROFILE",
    "LD_USE_LOAD_BIAS",
    "GLIBC_TUNABLES",
];
static PYTHON_RECEIPT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn python_success_receipt(script_rel: &str) -> String {
    let sequence = PYTHON_RECEIPT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "{PYTHON_SUCCESS_RECEIPT_SCHEMA}:{script_rel}:{}:{now}:{sequence}",
        std::process::id()
    )
}

fn has_python_success_receipt(status_success: bool, stderr: &str, expected_receipt: &str) -> bool {
    status_success
        && stderr
            .lines()
            .filter(|line| line.trim() == expected_receipt)
            .count()
            == 1
}

#[cfg(unix)]
fn trusted_python_interpreters() -> Result<Vec<PathBuf>, String> {
    use std::os::unix::fs::MetadataExt;

    let mut trusted = Vec::new();
    let mut rejected = Vec::new();
    for candidate in [
        Path::new("/usr/bin/python3"),
        Path::new("/usr/local/bin/python3"),
    ] {
        let canonical = match fs::canonicalize(candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                rejected.push(format!("{} is absent", candidate.display()));
                continue;
            }
            Err(error) => {
                rejected.push(format!(
                    "{} cannot be resolved ({error})",
                    candidate.display()
                ));
                continue;
            }
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(metadata) => metadata,
            Err(error) => {
                rejected.push(format!(
                    "{} cannot be inspected ({error})",
                    canonical.display()
                ));
                continue;
            }
        };
        let executable_is_trusted = metadata.is_file()
            && metadata.uid() == 0
            && metadata.mode() & 0o022 == 0
            && metadata.mode() & 0o6000 == 0
            && metadata.mode() & 0o111 != 0;
        let ancestry_is_trusted = canonical.ancestors().skip(1).all(|ancestor| {
            fs::metadata(ancestor).is_ok_and(|metadata| {
                metadata.is_dir() && metadata.uid() == 0 && metadata.mode() & 0o022 == 0
            })
        });
        if executable_is_trusted && ancestry_is_trusted {
            if !trusted.contains(&canonical) {
                trusted.push(canonical);
            }
        } else {
            rejected.push(format!(
                "{} is not a root-owned, non-group-writable, non-world-writable, non-set-id \
                 executable beneath root-controlled directories",
                canonical.display()
            ));
        }
    }
    if trusted.is_empty() {
        Err(format!(
            "no independently rooted Python interpreter is available ({})",
            rejected.join("; ")
        ))
    } else {
        Ok(trusted)
    }
}

#[cfg(not(unix))]
fn trusted_python_interpreters() -> Result<Vec<PathBuf>, String> {
    Err(String::from(
        "native execution has no independently verifiable Python interpreter trust root; \
         run the canonical Stone 0 gate inside WSL",
    ))
}

#[cfg(unix)]
fn trusted_git_executable() -> Result<PathBuf, String> {
    use std::os::unix::fs::MetadataExt;

    let mut rejected = Vec::new();
    for candidate in [Path::new("/usr/bin/git"), Path::new("/bin/git")] {
        let canonical = match fs::canonicalize(candidate) {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                rejected.push(format!("{} is absent", candidate.display()));
                continue;
            }
            Err(error) => {
                rejected.push(format!(
                    "{} cannot be resolved ({error})",
                    candidate.display()
                ));
                continue;
            }
        };
        let metadata = match fs::metadata(&canonical) {
            Ok(metadata) => metadata,
            Err(error) => {
                rejected.push(format!(
                    "{} cannot be inspected ({error})",
                    canonical.display()
                ));
                continue;
            }
        };
        let executable_is_trusted = metadata.is_file()
            && metadata.uid() == 0
            && metadata.mode() & 0o022 == 0
            && metadata.mode() & 0o6000 == 0
            && metadata.mode() & 0o111 != 0;
        let ancestry_is_trusted = canonical.ancestors().skip(1).all(|ancestor| {
            fs::metadata(ancestor).is_ok_and(|ancestor_metadata| {
                ancestor_metadata.is_dir()
                    && ancestor_metadata.uid() == 0
                    && ancestor_metadata.mode() & 0o022 == 0
            })
        });
        if executable_is_trusted && ancestry_is_trusted {
            return Ok(canonical);
        }
        rejected.push(format!(
            "{} is not a root-owned, non-group-writable, non-world-writable, non-set-id \
             executable beneath root-controlled directories",
            canonical.display()
        ));
    }
    Err(format!(
        "no independently rooted Git executable is available ({})",
        rejected.join("; ")
    ))
}

#[cfg(not(unix))]
fn trusted_git_executable() -> Result<PathBuf, String> {
    Err(String::from(
        "native execution has no independently verifiable Git trust root; \
         run the canonical Stone 0 gate inside WSL",
    ))
}

fn trusted_git_command(repository_root: &Path) -> Result<Command, String> {
    let git = trusted_git_executable()?;
    let mut command = Command::new(git);
    command
        .arg("-C")
        .arg(repository_root)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LC_ALL", "C");
    Ok(command)
}

#[cfg(unix)]
fn trusted_rust_tool_environment(
    repository_root: &Path,
) -> Result<(OsString, PathBuf, PathBuf), String> {
    use std::os::unix::fs::MetadataExt;

    let declared_cargo = std::env::var_os("CARGO")
        .map(PathBuf::from)
        .ok_or_else(|| "Cargo did not disclose the build tool that launched Stone 0".to_owned())?;
    if !declared_cargo.is_absolute() {
        return Err("Cargo disclosed a non-absolute executable path".to_owned());
    }
    let cargo = fs::canonicalize(&declared_cargo)
        .map_err(|error| format!("the launching Cargo executable cannot be resolved ({error})"))?;
    let tool_directory = cargo
        .parent()
        .ok_or_else(|| "the launching Cargo executable has no tool directory".to_owned())?;
    let rustc = fs::canonicalize(tool_directory.join("rustc"))
        .map_err(|error| format!("Cargo's sibling rustc cannot be resolved ({error})"))?;
    if cargo.file_name().and_then(|name| name.to_str()) != Some("cargo")
        || cargo.starts_with(repository_root)
        || rustc.starts_with(repository_root)
    {
        return Err(
            "the disclosed Rust toolchain is not an external cargo and sibling rustc pair"
                .to_owned(),
        );
    }
    let cargo_metadata = fs::metadata(&cargo)
        .map_err(|error| format!("the launching Cargo executable cannot be inspected ({error})"))?;
    let rustc_metadata = fs::metadata(&rustc)
        .map_err(|error| format!("Cargo's sibling rustc cannot be inspected ({error})"))?;
    let owner = cargo_metadata.uid();
    let executable_is_stable = |metadata: &fs::Metadata| {
        metadata.is_file()
            && metadata.uid() == owner
            && metadata.mode() & 0o002 == 0
            && metadata.mode() & 0o6000 == 0
            && metadata.mode() & 0o111 != 0
    };
    let ancestry_is_stable = cargo.ancestors().skip(1).all(|ancestor| {
        fs::metadata(ancestor).is_ok_and(|metadata| {
            metadata.is_dir()
                && (metadata.uid() == 0 || metadata.uid() == owner)
                && metadata.mode() & 0o002 == 0
        })
    });
    if !executable_is_stable(&cargo_metadata)
        || !executable_is_stable(&rustc_metadata)
        || !ancestry_is_stable
    {
        return Err(
            "the disclosed Rust toolchain is not a non-world-writable, non-set-id sibling pair \
             beneath owner- or root-controlled directories"
                .to_owned(),
        );
    }

    let mut path = tool_directory.as_os_str().to_os_string();
    path.push(":/usr/bin:/bin");
    Ok((path, cargo, rustc))
}

fn run_python_gate(root: &Path, script_rel: &str, args: &[&str]) -> ScriptResult {
    run_python_gate_with_environment(root, script_rel, args, &[])
}

fn run_python_gate_with_environment(
    root: &Path,
    script_rel: &str,
    args: &[&str],
    environment: &[(OsString, OsString)],
) -> ScriptResult {
    let path = root.join(script_rel);
    let canonical_root = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) => {
            return ScriptResult::Operational(format!(
                "could not canonicalize the repository root for {script_rel} ({error})"
            ));
        }
    };
    let canonical_path = match fs::canonicalize(&path) {
        Ok(path) => path,
        Err(error) => {
            return ScriptResult::Operational(format!(
                "{script_rel} is the declarative gate-authority runner but cannot be opened ({error}). \
                 A gate runner that is unavailable has not passed."
            ));
        }
    };
    if !canonical_path.starts_with(&canonical_root)
        || !canonical_path.is_file()
        || fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        // A LISTED SCRIPT THAT IS ABSENT IS A FAILURE, not a skip. This list names what MUST run, so a
        // missing or redirected entry means either the gate was deleted, the path drifted, or its bytes
        // escaped the repository boundary, and every case should be loud.
        // Skipping made deleting a gate the quietest way to stop it convicting.
        return ScriptResult::Operational(format!(
            "{script_rel} does not resolve to a regular repository-owned script. A redirected gate \
             runner has not passed; restore it deliberately."
        ));
    }
    // The interpreter is part of the authority boundary. Resolving it through caller-controlled PATH
    // would let a shim read and replay the wrapper receipt. Canonical Unix execution therefore accepts
    // only fixed absolute candidates whose executable and complete directory ancestry are controlled by
    // root and are not group- or world-writable. Native Windows fails closed and uses the documented WSL
    // route until it gains an equivalent independently verifiable trust root.
    let interpreters = match trusted_python_interpreters() {
        Ok(interpreters) => interpreters,
        Err(error) => return ScriptResult::Operational(format!("{script_rel}: {error}")),
    };
    #[cfg(unix)]
    let (trusted_path, trusted_cargo, trusted_rustc) =
        match trusted_rust_tool_environment(&canonical_root) {
            Ok(environment) => environment,
            Err(error) => return ScriptResult::Operational(format!("{script_rel}: {error}")),
        };

    let mut unavailable = Vec::new();
    let mut selected = None;
    for program in interpreters {
        let expected_receipt = python_success_receipt(script_rel);
        let mut command = Command::new(&program);
        command
            .args(["-I", "-E", "-s", "-S", "-B", "-c", PYTHON_WRAPPER])
            .arg(&expected_receipt)
            .arg(&canonical_path)
            .args(args)
            .current_dir(&canonical_root)
            .envs(environment.iter().cloned())
            .env("PYTHONSAFEPATH", "1")
            .env("PYTHONNOUSERSITE", "1")
            .env("PYTHONDONTWRITEBYTECODE", "1");
        #[cfg(unix)]
        command
            .env("PATH", &trusted_path)
            .env("CARGO", &trusted_cargo)
            .env("RUSTC", &trusted_rustc);
        for key in SCRUBBED_PYTHON_ENVIRONMENT {
            command.env_remove(key);
        }
        let result = command.output();
        match result {
            Ok(out) => {
                selected = Some((out, expected_receipt));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                unavailable.push(format!("{} was not found", program.display()));
            }
            Err(e) => unavailable.push(format!("{} could not spawn ({e})", program.display())),
        }
    }
    let Some((out, expected_receipt)) = selected else {
        return ScriptResult::Operational(format!(
            "could not run a Python interpreter for {script_rel} ({}); skipped",
            unavailable.join("; ")
        ));
    };
    let stderr = String::from_utf8_lossy(&out.stderr);
    if has_python_success_receipt(out.status.success(), &stderr, &expected_receipt) {
        return ScriptResult::Clean;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let policy_detection = has_policy_detection_protocol(out.status.code(), &stdout, &stderr);
    let details = match (stdout.trim(), stderr.trim()) {
        ("", "") if out.status.success() => String::from(
            "gate runner exited zero without its invocation-bound Python success receipt",
        ),
        ("", "") => String::from("gate runner exited nonzero without output"),
        (stdout, "") => stdout.to_string(),
        ("", stderr) => stderr.to_string(),
        (stdout, stderr) => format!("{stdout}\n{stderr}"),
    };
    if policy_detection {
        ScriptResult::Detected(format!("{script_rel}:\n{details}"))
    } else {
        ScriptResult::Operational(format!(
            "{script_rel} exited without the required success receipt or policy-detection protocol marker:\n{details}"
        ))
    }
}

fn has_policy_detection_protocol(exit_code: Option<i32>, stdout: &str, stderr: &str) -> bool {
    exit_code == Some(1)
        && stdout
            .lines()
            .chain(stderr.lines())
            .any(|line| line.trim() == POLICY_DETECTION_MARKER)
}

fn validate_mandatory_authority_gate_manifest(raw: &str) -> Result<(), String> {
    let normalized = raw.replace("\r\n", "\n");
    let blocks: Vec<String> = normalized
        .split("[[gate]]")
        .skip(1)
        .map(|body| format!("[[gate]]{body}"))
        .filter(|block| block.contains("\nid = \"canonical.authority-watchdog\"\n"))
        .collect();
    if blocks.len() != 1 {
        return Err(format!(
            "{GATE_MANIFEST_PATH} must contain exactly one canonical.authority-watchdog block; found {}",
            blocks.len()
        ));
    }
    if blocks[0].trim() != MANDATORY_AUTHORITY_GATE_BLOCK.trim() {
        return Err(format!(
            "{GATE_MANIFEST_PATH} canonical.authority-watchdog block differs from the independent Stone 0 bootstrap pin"
        ));
    }
    Ok(())
}

fn verify_mandatory_authority_gate(root: &Path) -> Result<(), String> {
    let path = root.join(GATE_MANIFEST_PATH);
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("could not read {} ({error})", path.display()))?;
    validate_mandatory_authority_gate_manifest(&raw)
}

fn provenance_scan_with(
    root: &Path,
    mut execute: impl FnMut(&Path, &str, &[&str]) -> ScriptResult,
) -> ProvenanceOutcome {
    let mut override_eligible_detections = Vec::new();
    let mut non_overridable_failures = Vec::new();
    if let Err(error) = verify_mandatory_authority_gate(root) {
        non_overridable_failures.push(format!(
            "canonical provenance runner unavailable: {error}. A gate that cannot run has not passed."
        ));
        return ProvenanceOutcome {
            override_eligible_detections,
            non_overridable_failures,
        };
    }
    let (script, args) = PROVENANCE_RUNNER;
    match execute(root, script, args) {
        ScriptResult::Clean => {}
        ScriptResult::Detected(r) => override_eligible_detections.push(r),
        ScriptResult::Operational(w) => non_overridable_failures.push(format!(
            "canonical provenance runner unavailable: {w}. A gate that cannot run has not passed."
        )),
    }
    // Execute every authority bootstrap directly as a second path. The
    // declarative runner cannot suppress these calls by returning success or
    // omitting a manifest entry. These calls enforce the authority boundary
    // itself, so neither a semantic detection nor an operational failure may
    // enter the owner-override bucket.
    for (authority_script, authority_args) in MANDATORY_AUTHORITY_COMMANDS {
        match execute(root, authority_script, authority_args) {
            ScriptResult::Clean => {}
            ScriptResult::Detected(r) => non_overridable_failures.push(format!(
                "mandatory authority command {authority_script} detected a policy violation:\n{r}"
            )),
            ScriptResult::Operational(w) => non_overridable_failures.push(format!(
                "mandatory authority command {authority_script} unavailable: {w}. A gate that cannot run has not passed."
            )),
        }
    }
    ProvenanceOutcome {
        override_eligible_detections,
        non_overridable_failures,
    }
}

fn provenance_scan(root: &Path) -> ProvenanceOutcome {
    provenance_scan_with(root, run_python_gate)
}

// ----------------------------------------------------------------------------------------------------
// Worktree and index gathering (git). Every step fails open: any error returns Err and the caller
// warns and skips, never blocks.
// ----------------------------------------------------------------------------------------------------

fn git_worktree_files(root: &Path) -> Result<Vec<RepoFile>, String> {
    let mut command =
        trusted_git_command(root).map_err(|error| format!("trusted Git unavailable ({error})"))?;
    let out = command
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .output()
        .map_err(|e| format!("git ls-files did not spawn ({e})"))?;
    if !out.status.success() {
        return Err("git ls-files exited non-zero".to_string());
    }
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    for rec in out.stdout.split(|&b| b == 0) {
        if rec.is_empty() {
            continue;
        }
        let rel = String::from_utf8_lossy(rec).into_owned();
        if !seen.insert(rel.clone()) {
            continue;
        }
        match std::fs::read(root.join(&rel)) {
            Ok(bytes) => files.push(RepoFile { path: rel, bytes }),
            Err(_) => {
                // A listed path with no readable worktree file can be a submodule gitlink or a deleted
                // tracked path. Skip it; the index scan still covers any staged blob.
            }
        }
    }
    Ok(files)
}

/// Read the staged blob content of every tracked path via a single `git cat-file --batch`. The blob
/// hashes are fed on the child's stdin (a writer thread avoids a pipe deadlock); no secret ever touches
/// a command line. Paths are prefixed `(index)` so a report distinguishes a staged hit from a worktree
/// hit.
fn git_index_blobs(root: &Path) -> Result<Vec<RepoFile>, String> {
    let mut listing_command =
        trusted_git_command(root).map_err(|error| format!("trusted Git unavailable ({error})"))?;
    let out = listing_command
        .args(["ls-files", "-s", "-z"])
        .output()
        .map_err(|e| format!("git ls-files -s did not spawn ({e})"))?;
    if !out.status.success() {
        return Err("git ls-files -s exited non-zero".to_string());
    }
    let mut sha_to_path: BTreeMap<String, String> = BTreeMap::new();
    for rec in out.stdout.split(|&b| b == 0) {
        if rec.is_empty() {
            continue;
        }
        let s = String::from_utf8_lossy(rec);
        if let Some((meta, path)) = s.split_once('\t') {
            let parts: Vec<&str> = meta.split_whitespace().collect();
            if parts.len() >= 2 {
                sha_to_path
                    .entry(parts[1].to_string())
                    .or_insert_with(|| path.to_string());
            }
        }
    }
    if sha_to_path.is_empty() {
        return Ok(Vec::new());
    }
    let shas: Vec<String> = sha_to_path.keys().cloned().collect();
    let mut cat_file_command =
        trusted_git_command(root).map_err(|error| format!("trusted Git unavailable ({error})"))?;
    let mut child = cat_file_command
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("git cat-file did not spawn ({e})"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file stdin unavailable")?;
    let writer = std::thread::spawn(move || {
        for sha in shas {
            if writeln!(stdin, "{sha}").is_err() {
                break;
            }
        }
    });
    let mut stdout = child
        .stdout
        .take()
        .ok_or("git cat-file stdout unavailable")?;
    let mut buf = Vec::new();
    stdout
        .read_to_end(&mut buf)
        .map_err(|e| format!("reading git cat-file ({e})"))?;
    let _ = writer.join();
    let _ = child.wait();

    let mut files = Vec::new();
    let mut i = 0usize;
    while i < buf.len() {
        let Some(off) = buf[i..].iter().position(|&b| b == b'\n') else {
            break;
        };
        let nl = i + off;
        let header = String::from_utf8_lossy(&buf[i..nl]).into_owned();
        i = nl + 1;
        let parts: Vec<&str> = header.split(' ').collect();
        if parts.len() == 2 && parts[1] == "missing" {
            continue;
        }
        if parts.len() < 3 {
            continue;
        }
        let size: usize = match parts[2].trim().parse() {
            Ok(n) => n,
            Err(_) => break,
        };
        if i + size > buf.len() {
            break;
        }
        let content = buf[i..i + size].to_vec();
        i += size;
        if i < buf.len() && buf[i] == b'\n' {
            i += 1;
        }
        if let Some(path) = sha_to_path.get(parts[0]) {
            files.push(RepoFile {
                path: format!("(index) {path}"),
                bytes: content,
            });
        }
    }
    Ok(files)
}

// ----------------------------------------------------------------------------------------------------
// The driver.
// ----------------------------------------------------------------------------------------------------

fn override_env_excludes() -> Vec<String> {
    vec![
        // The gate's own source and scripts, and the design doc, legitimately name the variable.
        "crates/stone0/".to_string(),
        "scripts/stone0".to_string(),
        "docs/working/Q1_STONE0_PROVENANCE_GATE_DESIGN.md".to_string(),
    ]
}

fn read_secrets(cfg: &GateConfig, notices: &mut Vec<String>) -> (Option<String>, Option<String>) {
    if cfg.mode != Mode::Local {
        return (None, None);
    }
    match std::fs::read_to_string(&cfg.secrets_path) {
        Ok(content) => {
            let (pw, canary) = parse_secrets(&content);
            let pw = if pw.is_empty() { None } else { Some(pw) };
            (pw, canary)
        }
        Err(_) => {
            notices.push(format!(
                "secrets file {} not present; the live-password, canary, and override checks are skipped (the owner creates it out of band)",
                cfg.secrets_path.display()
            ));
            (None, None)
        }
    }
}

fn read_tombstones(cfg: &GateConfig, warnings: &mut Vec<String>) -> Vec<String> {
    match std::fs::read_to_string(&cfg.tombstones_path) {
        Ok(content) => parse_tombstones(&content),
        Err(_) => {
            warnings.push(format!(
                "tombstone list {} not present; the tombstone scan is skipped",
                cfg.tombstones_path.display()
            ));
            Vec::new()
        }
    }
}

fn dedup(mut v: Vec<String>) -> Vec<String> {
    let set: BTreeSet<String> = v.drain(..).collect();
    set.into_iter().collect()
}

/// Run every Stone 0 check for the given configuration and return the verdict.
///
/// The canonical provenance runner is mandatory. Its operational failure is a
/// build failure, not a skipped check.
pub fn gate(cfg: &GateConfig) -> GateReport {
    let mut failures: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut notices: Vec<String> = Vec::new();

    if cfg.override_value.is_some() && cfg.secrets_path != Path::new(DEFAULT_SECRETS_PATH) {
        failures.push(
            "a caller-selected secrets path cannot authorize an override; use the documented default owner trust path"
                .to_owned(),
        );
    }

    // Gather tracked and nonignored untracked worktree files once; the laundering, tombstone,
    // override-env, and canary scans share them.
    let files = match git_worktree_files(&cfg.repo_root) {
        Ok(f) => Some(f),
        Err(e) => {
            warnings.push(format!(
                "git worktree-file scan unavailable ({e}); the laundering, tombstone, override-env, and canary scans are skipped"
            ));
            None
        }
    };

    // Check 1: the provenance scan (both modes).
    let prov = provenance_scan(&cfg.repo_root);
    for hard_failure in &prov.non_overridable_failures {
        failures.push(hard_failure.clone());
    }
    let provenance_failed = !prov.override_eligible_detections.is_empty();

    // Secrets (local mode only).
    let (password, canary) = read_secrets(cfg, &mut notices);

    // Check 4: the override-env-name scan (both modes).
    if let Some(files) = &files {
        for h in find_override_env_hits(files, &override_env_excludes()) {
            failures.push(format!(
                "override-env baked into a committed file: {h} assigns {OVERRIDE_ENV}. The override is a single-command environment variable, never committed; remove the assignment."
            ));
        }
    }

    // Check 3: the tombstone scan (both modes).
    if let Some(files) = &files {
        let tombs = read_tombstones(cfg, &mut warnings);
        if !tombs.is_empty() {
            let exclude = vec![TOMBSTONE_REL.to_string()];
            for (path, _phrase) in find_tombstone_hits(files, &tombs, &exclude) {
                failures.push(format!(
                    "laundered retired override phrase found in {path}: a tombstoned phrase must appear nowhere but the tombstone list. A retired-but-present copy is a stale laundered credential; remove it."
                ));
            }
        }
    }

    // Checks 2 and 5: the live-password laundering scan and the canary (local mode only).
    if cfg.mode == Mode::Local {
        if let (Some(files), Some(pw)) = (&files, password.as_ref()) {
            let mut hits = find_password_hits(files, pw);
            match git_index_blobs(&cfg.repo_root) {
                Ok(idx) => hits.extend(find_password_hits(&idx, pw)),
                Err(e) => warnings.push(format!(
                    "git index scan unavailable ({e}); the git index was not scanned for the live password"
                )),
            }
            for h in dedup(hits) {
                failures.push(format!(
                    "laundered credential: the live override password appears in {h}. The password value is not printed. Remove it immediately, and Nathan must rotate the password."
                ));
            }
        }
        if let (Some(files), Some(can)) = (&files, canary.as_ref()) {
            for h in find_canary_hits(files, can, &[]) {
                failures.push(format!(
                    "canary tripped: the secrets-file canary appears in {h}, proof the secrets file was read and its content committed. Rotate the canary and investigate."
                ));
            }
        }
    }

    // Check 1 verdict, with the override path (local mode only).
    let mut overridden = false;
    if provenance_failed {
        let mut report =
            String::from("provenance scan found un-provenanced or fixture value(s):\n");
        for d in &prov.override_eligible_detections {
            report.push_str(d);
            report.push('\n');
        }
        let valid_override = cfg.mode == Mode::Local
            && override_is_valid(
                cfg.override_value.as_deref(),
                password.as_deref().unwrap_or(""),
            );
        if valid_override {
            overridden = true;
            notices.push("STONE 0 OVERRIDDEN BY OWNER PASSWORD".to_string());
            notices.push(report);
        } else {
            failures.push(format!("{report}\n{OVERRIDE_INSTRUCTIONS}"));
        }
    } else if cfg.mode == Mode::Local && cfg.override_value.is_some() {
        notices.push(
            "note: STONE0_OVERRIDE was set but the provenance scan was clean; the override was unused"
                .to_string(),
        );
    }

    let verdict = if !failures.is_empty() {
        Verdict::Fail
    } else if overridden {
        Verdict::Overridden
    } else {
        Verdict::Clean
    };

    GateReport {
        verdict,
        failures,
        warnings,
        notices,
    }
}

// ----------------------------------------------------------------------------------------------------
// Repository binding and binary entry points.
// ----------------------------------------------------------------------------------------------------

const REPOSITORY_ROOT_MEMBERS: [&str; 6] = [
    "Cargo.toml",
    "Cargo.lock",
    "crates/stone0/Cargo.toml",
    "scripts/gates.toml",
    "scripts/gate_runner.py",
    TOMBSTONE_REL,
];

fn validate_regular_repository_member(root: &Path, relative: &str) -> Result<(), String> {
    let mut path = root.to_path_buf();
    for component in Path::new(relative).components() {
        let std::path::Component::Normal(name) = component else {
            return Err(format!(
                "repository identity member has a non-normal path: {relative}"
            ));
        };
        path.push(name);
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            format!(
                "repository identity member {} is unavailable ({error})",
                path.display()
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "repository identity member traverses a symbolic link: {}",
                path.display()
            ));
        }
    }
    let canonical = fs::canonicalize(&path).map_err(|error| {
        format!(
            "repository identity member {} cannot be canonicalized ({error})",
            path.display()
        )
    })?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return Err(format!(
            "repository identity member is not a regular file beneath the bound root: {relative}"
        ));
    }
    Ok(())
}

fn trusted_git_top_level(root: &Path) -> Result<PathBuf, String> {
    let mut command = trusted_git_command(root)?;
    let output = command
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| format!("trusted Git root check did not spawn ({error})"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if detail.is_empty() {
            "trusted Git root check exited nonzero without diagnostics".to_owned()
        } else {
            format!("trusted Git root check exited nonzero ({detail})")
        });
    }
    let reported = String::from_utf8(output.stdout)
        .map_err(|_| "trusted Git returned a non-UTF-8 repository root".to_owned())?;
    let reported = reported.trim();
    if reported.is_empty() {
        return Err("trusted Git returned an empty repository root".to_owned());
    }
    fs::canonicalize(reported)
        .map_err(|error| format!("trusted Git repository root cannot be canonicalized ({error})"))
}

fn canonicalize_and_validate_repository_root(root: &Path) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(root).map_err(|error| {
        format!(
            "repository root {} cannot be canonicalized ({error})",
            root.display()
        )
    })?;
    if !canonical.is_dir() {
        return Err(format!(
            "repository root is not a directory: {}",
            canonical.display()
        ));
    }
    for relative in REPOSITORY_ROOT_MEMBERS {
        validate_regular_repository_member(&canonical, relative)?;
    }

    let workspace_manifest = fs::read_to_string(canonical.join("Cargo.toml"))
        .map_err(|error| format!("workspace manifest cannot be read ({error})"))?;
    let stone0_manifest = fs::read_to_string(canonical.join("crates/stone0/Cargo.toml"))
        .map_err(|error| format!("Stone 0 manifest cannot be read ({error})"))?;
    if !workspace_manifest
        .lines()
        .any(|line| line.trim() == "[workspace]")
        || !workspace_manifest.contains("\"crates/stone0\"")
        || !stone0_manifest
            .lines()
            .any(|line| line.trim() == "name = \"civsim-stone0\"")
    {
        return Err(
            "repository root does not carry the expected workspace and Stone 0 identities"
                .to_owned(),
        );
    }

    let git_root = trusted_git_top_level(&canonical)?;
    if git_root != canonical {
        return Err(format!(
            "explicit repository root {} does not equal trusted Git top level {}",
            canonical.display(),
            git_root.display()
        ));
    }
    Ok(canonical)
}

fn has_repository_root_shape(candidate: &Path) -> bool {
    [
        "Cargo.toml",
        "crates/stone0/Cargo.toml",
        "scripts/gates.toml",
    ]
    .iter()
    .all(|relative| candidate.join(relative).is_file())
}

fn discover_repository_root_from(start: &Path) -> Result<PathBuf, String> {
    let canonical_start = fs::canonicalize(start).map_err(|error| {
        format!(
            "repository-root search start {} cannot be canonicalized ({error})",
            start.display()
        )
    })?;
    let start_directory = if canonical_start.is_dir() {
        canonical_start
    } else {
        canonical_start
            .parent()
            .ok_or_else(|| "repository-root search start has no parent directory".to_owned())?
            .to_path_buf()
    };
    let mut rejected = Vec::new();
    for candidate in start_directory.ancestors() {
        if has_repository_root_shape(candidate) {
            match canonicalize_and_validate_repository_root(candidate) {
                Ok(root) => return Ok(root),
                Err(error) => rejected.push(format!("{} ({error})", candidate.display())),
            }
        }
    }
    if rejected.is_empty() {
        Err(format!(
            "no repository root was found above {}",
            start_directory.display()
        ))
    } else {
        Err(format!(
            "no valid repository root was found above {}: {}",
            start_directory.display(),
            rejected.join("; ")
        ))
    }
}

fn resolve_cli_repository_root() -> Result<PathBuf, String> {
    let current_directory = std::env::current_dir()
        .map_err(|error| format!("current directory is unavailable ({error})"))?;
    match discover_repository_root_from(&current_directory) {
        Ok(root) => Ok(root),
        Err(current_error) => {
            let executable = std::env::current_exe().map_err(|error| {
                format!("{current_error}; current executable is unavailable ({error})")
            })?;
            discover_repository_root_from(&executable).map_err(|executable_error| {
                format!(
                    "repository root could not be resolved from the current directory or executable: \
                     current directory: {current_error}; executable: {executable_error}"
                )
            })
        }
    }
}

fn emit_report(report: &GateReport) {
    for w in &report.warnings {
        eprintln!("stone0: warning: {w}");
    }
    for n in &report.notices {
        if n == "STONE 0 OVERRIDDEN BY OWNER PASSWORD" {
            eprintln!("\n================================================================");
            eprintln!("  STONE 0 OVERRIDDEN BY OWNER PASSWORD");
            eprintln!("  The provenance gate found a violation, but a valid owner");
            eprintln!("  override was supplied. Allowing this one build.");
            eprintln!("================================================================\n");
        } else {
            eprintln!("stone0: {n}");
        }
    }
    for f in &report.failures {
        eprintln!("stone0: FAIL: {f}");
    }
    match report.verdict {
        Verdict::Clean => eprintln!("stone0: clean (every check passed)"),
        Verdict::Overridden => {
            eprintln!("stone0: allowed by owner override (the provenance violation stands, recorded above)")
        }
        Verdict::Fail => {
            eprintln!("stone0: BLOCKED (a positive detection; see the failures above)")
        }
    }
}

fn run_at_canonical_repository_root(mode: Mode, repo_root: PathBuf) -> i32 {
    let override_value = if mode == Mode::Local {
        std::env::var(OVERRIDE_ENV).ok()
    } else {
        None
    };
    let secrets_path = match select_secrets_path(
        std::env::var("STONE0_SECRETS_PATH").ok().as_deref(),
        override_value.as_deref(),
    ) {
        Ok(path) => path,
        Err(error) => {
            eprintln!("stone0: FAIL: {error}");
            return 1;
        }
    };
    let tombstones_path = repo_root.join(TOMBSTONE_REL);
    let cfg = GateConfig {
        repo_root,
        secrets_path,
        tombstones_path,
        mode,
        override_value,
    };
    let report = gate(&cfg);
    emit_report(&report);
    report.exit_code()
}

/// Run Stone 0 against one caller-bound repository root.
///
/// The path is canonicalized, checked against repository identity members, and
/// required to equal the top level reported by an independently rooted Git
/// executable with a scrubbed environment. Any mismatch or unavailable trust
/// root fails closed before a gate mode runs.
pub fn run_at_repository_root(mode: Mode, repo_root: &Path) -> i32 {
    let repo_root = match canonicalize_and_validate_repository_root(repo_root) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("stone0: FAIL: repository-root binding refused: {error}");
            return 1;
        }
    };
    if mode == Mode::SelfTest {
        return self_test();
    }
    run_at_canonical_repository_root(mode, repo_root)
}

/// Resolve the repository without invoking ambient `git`, run the gate, print
/// the report, and return the process exit code.
///
/// `SelfTest` is repository-independent. Build scripts must use
/// [`run_at_repository_root`] so their authority root comes from the calling
/// build anchor rather than process location.
pub fn run(mode: Mode) -> i32 {
    if mode == Mode::SelfTest {
        return self_test();
    }
    let repo_root = match resolve_cli_repository_root() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("stone0: FAIL: repository-root resolution refused: {error}");
            return 1;
        }
    };
    run_at_canonical_repository_root(mode, repo_root)
}

fn select_secrets_path(
    configured_path: Option<&str>,
    override_value: Option<&str>,
) -> Result<PathBuf, String> {
    match configured_path {
        Some(path) if path.trim().is_empty() => {
            Err("STONE0_SECRETS_PATH is empty; refusing an ambiguous secrets authority".to_owned())
        }
        Some(_) if override_value.is_some() => Err(
            "STONE0_SECRETS_PATH and STONE0_OVERRIDE cannot be supplied together; a caller-selected secrets file cannot authorize that caller's override"
                .to_owned(),
        ),
        Some(path) => Ok(PathBuf::from(path)),
        None => Ok(PathBuf::from(DEFAULT_SECRETS_PATH)),
    }
}

// ----------------------------------------------------------------------------------------------------
// The self-test: prove each detector fires (and does not misfire) on synthetic fixtures. Mirrors the
// `--self-test` convention of the python gates.
// ----------------------------------------------------------------------------------------------------

fn self_test() -> i32 {
    let mut problems: Vec<String> = Vec::new();
    let mut check = |name: &str, ok: bool| {
        if !ok {
            problems.push(name.to_string());
        }
    };

    // base64 known vectors (RFC 4648).
    check("base64 of empty", base64_standard(b"").is_empty());
    check("base64 of 'f'", base64_standard(b"f") == "Zg==");
    check("base64 of 'fo'", base64_standard(b"fo") == "Zm8=");
    check("base64 of 'hello'", base64_standard(b"hello") == "aGVsbG8=");

    // override comparison accepts the right password, rejects a wrong one, an absent one, an empty one.
    check(
        "override accepts match",
        override_is_valid(Some("s3cret pass"), "s3cret pass"),
    );
    check(
        "override rejects mismatch",
        !override_is_valid(Some("wrong"), "s3cret pass"),
    );
    check(
        "override rejects absent",
        !override_is_valid(None, "s3cret pass"),
    );
    check(
        "override rejects empty password",
        !override_is_valid(Some(""), ""),
    );
    check(
        "caller-selected secrets cannot authorize caller override",
        select_secrets_path(Some("synthetic.pass"), Some("synthetic override")).is_err(),
    );
    check(
        "custom secrets path remains usable without an override",
        select_secrets_path(Some("synthetic.pass"), None) == Ok(PathBuf::from("synthetic.pass")),
    );
    check(
        "empty secrets path is rejected",
        select_secrets_path(Some(""), None).is_err(),
    );
    check(
        "marked policy detection is override-eligible",
        has_policy_detection_protocol(Some(1), "", POLICY_DETECTION_MARKER),
    );
    check(
        "unmarked nonzero exit is authority failure",
        !has_policy_detection_protocol(Some(1), "gate failed", ""),
    );
    check(
        "wrong exit code cannot claim policy detection",
        !has_policy_detection_protocol(Some(2), POLICY_DETECTION_MARKER, ""),
    );
    let python_receipt = "civsim.stone0.python-authority-success.v1:fixture";
    check(
        "exact Python success receipt accepted",
        has_python_success_receipt(true, python_receipt, python_receipt),
    );
    check(
        "missing Python success receipt fails closed",
        !has_python_success_receipt(true, "", python_receipt),
    );
    check(
        "duplicate Python success receipt fails closed",
        !has_python_success_receipt(
            true,
            &format!("{python_receipt}\n{python_receipt}"),
            python_receipt,
        ),
    );
    check(
        "nonzero Python exit cannot claim success",
        !has_python_success_receipt(false, python_receipt, python_receipt),
    );
    check(
        "wrong Python success receipt fails closed",
        !has_python_success_receipt(
            true,
            "civsim.stone0.python-authority-success.v1:other",
            python_receipt,
        ),
    );
    check(
        "mandatory authority commands retain the independent direct path",
        MANDATORY_AUTHORITY_COMMANDS
            == [
                ("scripts/authority_watchdog_gate.py", &[] as &[&str]),
                ("scripts/codata_floor_evidence_gate.py", &[] as &[&str]),
                ("scripts/stone0_build_wiring_gate.py", &[] as &[&str]),
                ("scripts/fixed_math_authority_gate.py", &[] as &[&str]),
                ("scripts/external_claim_gate.py", &[] as &[&str]),
            ],
    );

    let exact_gate = format!("{MANDATORY_AUTHORITY_GATE_BLOCK}\n");
    check(
        "mandatory authority gate exact block accepted",
        validate_mandatory_authority_gate_manifest(&exact_gate).is_ok(),
    );
    for (label, old, replacement) in [
        (
            "mandatory authority id mutation caught",
            "id = \"canonical.authority-watchdog\"",
            "id = \"canonical.authority-watchdog-weakened\"",
        ),
        (
            "mandatory authority order mutation caught",
            "order = 65",
            "order = 64",
        ),
        (
            "mandatory authority description mutation caught",
            "description = \"Require independent pairs for active authority-bearing mechanical claims.\"",
            "description = \"Weakened authority claim.\"",
        ),
        (
            "mandatory authority tiers mutation caught",
            "tiers = [\"canonical\", \"doctor\", \"pr\", \"full\", \"nightly\", \"stop\"]",
            "tiers = [\"canonical\", \"doctor\", \"pr\", \"full\", \"nightly\"]",
        ),
        (
            "mandatory authority phase mutation caught",
            "phase = \"provenance\"",
            "phase = \"post\"",
        ),
        (
            "mandatory authority command mutation caught",
            "command = [\"{python}\", \"scripts/authority_watchdog_gate.py\"]",
            "command = [\"{python}\", \"scripts/authority_watchdog_gate.py\", \"--weakened\"]",
        ),
        (
            "mandatory authority self-test mutation caught",
            "self_test = [\"{python}\", \"scripts/authority_watchdog_gate.py\", \"--self-test\"]",
            "self_test = [\"{python}\", \"scripts/authority_watchdog_gate.py\"]",
        ),
        (
            "mandatory authority timeout mutation caught",
            "timeout_seconds = 120",
            "timeout_seconds = 1",
        ),
        (
            "mandatory authority cache mutation caught",
            "cache = \"content-hash\"",
            "cache = \"never\"",
        ),
        (
            "mandatory authority no-cache metadata mutation caught",
            "cache = \"content-hash\"",
            "cache = \"content-hash\"\nno_cache_reason = \"bypass\"",
        ),
        (
            "mandatory authority input mutation caught",
            "  \"scripts/gate_runner.py\",",
            "  \"scripts/gate_runner-weakened.py\",",
        ),
        (
            "mandatory authority path-trigger mutation caught",
            "  \"crates/**\",",
            "  \"crates/units/**\",",
        ),
    ] {
        let changed = exact_gate.replacen(old, replacement, 1);
        check(
            label,
            changed != exact_gate
                && validate_mandatory_authority_gate_manifest(&changed).is_err(),
        );
    }
    check(
        "mandatory authority removal caught",
        validate_mandatory_authority_gate_manifest("").is_err(),
    );
    let duplicate_gate = format!("{exact_gate}{exact_gate}");
    check(
        "mandatory authority duplicate caught",
        validate_mandatory_authority_gate_manifest(&duplicate_gate).is_err(),
    );

    let integration_root = std::env::temp_dir().join(format!(
        "civsim-stone0-authority-self-test-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&integration_root);
    let integration = (|| -> Result<
        (
            Vec<String>,
            ProvenanceOutcome,
            ProvenanceOutcome,
            ProvenanceOutcome,
        ),
        String,
    > {
        fs::create_dir_all(integration_root.join("scripts"))
            .map_err(|error| format!("could not create integration fixture: {error}"))?;
        fs::write(
            integration_root.join(GATE_MANIFEST_PATH),
            MANDATORY_AUTHORITY_GATE_BLOCK,
        )
        .map_err(|error| format!("could not write integration fixture: {error}"))?;
        let mut observed = Vec::new();
        let outcome = provenance_scan_with(&integration_root, |_root, script, args| {
            let mut command = vec![script.to_owned()];
            command.extend(args.iter().map(|argument| (*argument).to_owned()));
            observed.push(command.join(" "));
            if script == "scripts/fixed_math_authority_gate.py" {
                ScriptResult::Operational("synthetic direct authority failure".to_owned())
            } else {
                ScriptResult::Clean
            }
        });
        let detected_outcome =
            provenance_scan_with(&integration_root, |_root, script, _args| {
                if script == "scripts/fixed_math_authority_gate.py" {
                    ScriptResult::Detected("synthetic direct authority detection".to_owned())
                } else {
                    ScriptResult::Clean
                }
            });
        let ordinary_detection_outcome =
            provenance_scan_with(&integration_root, |_root, script, _args| {
                if script == PROVENANCE_RUNNER.0 {
                    ScriptResult::Detected("synthetic ordinary provenance detection".to_owned())
                } else {
                    ScriptResult::Clean
                }
            });
        Ok((
            observed,
            outcome,
            detected_outcome,
            ordinary_detection_outcome,
        ))
    })();
    let _ = fs::remove_dir_all(&integration_root);
    match integration {
        Ok((observed, outcome, detected_outcome, ordinary_detection_outcome)) => {
            check(
                "provenance scan executes the runner and every direct authority command",
                observed
                    == [
                        "scripts/gate_runner.py run --tier canonical --phase provenance",
                        "scripts/authority_watchdog_gate.py",
                        "scripts/codata_floor_evidence_gate.py",
                        "scripts/stone0_build_wiring_gate.py",
                        "scripts/fixed_math_authority_gate.py",
                        "scripts/external_claim_gate.py",
                    ],
            );
            check(
                "direct authority operational failure propagates closed",
                outcome.override_eligible_detections.is_empty()
                    && outcome.non_overridable_failures
                        == ["mandatory authority command scripts/fixed_math_authority_gate.py unavailable: synthetic direct authority failure. A gate that cannot run has not passed.".to_owned()],
            );
            check(
                "direct authority detection is never override eligible",
                detected_outcome.override_eligible_detections.is_empty()
                    && detected_outcome.non_overridable_failures
                        == ["mandatory authority command scripts/fixed_math_authority_gate.py detected a policy violation:\nsynthetic direct authority detection".to_owned()],
            );
            check(
                "ordinary provenance detection remains override eligible",
                ordinary_detection_outcome.override_eligible_detections
                    == ["synthetic ordinary provenance detection".to_owned()]
                    && ordinary_detection_outcome
                        .non_overridable_failures
                        .is_empty(),
            );
        }
        Err(error) => check(
            &format!("direct authority integration fixture failed: {error}"),
            false,
        ),
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let manifest_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root_binding_fixture = manifest_root.join(format!(
            ".civsim-stone0-root-binding-self-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root_binding_fixture);
        let root_binding = (|| -> Result<(PathBuf, PathBuf, bool, bool, bool, bool), String> {
            let fake_bin = root_binding_fixture.join("bin");
            let fake_git = fake_bin.join("git");
            let fake_git_sentinel = fake_bin.join("ambient-git-ran");
            fs::create_dir_all(&fake_bin)
                .map_err(|error| format!("could not create root-binding fixture: {error}"))?;
            fs::write(
                &fake_git,
                "#!/bin/sh\n\
                 : > \"${0%/*}/ambient-git-ran\"\n\
                 printf '/tmp/redirected-stone0-root\\n'\n\
                 exit 0\n",
            )
            .map_err(|error| format!("could not write fake Git: {error}"))?;
            let mut permissions = fs::metadata(&fake_git)
                .map_err(|error| format!("could not inspect fake Git: {error}"))?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&fake_git, permissions)
                .map_err(|error| format!("could not arm fake Git: {error}"))?;

            let decoy_root = root_binding_fixture.join("decoy-root");
            fs::create_dir_all(decoy_root.join("crates/stone0"))
                .and_then(|_| fs::create_dir_all(decoy_root.join("scripts")))
                .and_then(|_| {
                    fs::write(
                        decoy_root.join("Cargo.toml"),
                        "[workspace]\nmembers = [\"crates/stone0\"]\n",
                    )
                })
                .and_then(|_| fs::write(decoy_root.join("Cargo.lock"), ""))
                .and_then(|_| {
                    fs::write(
                        decoy_root.join("crates/stone0/Cargo.toml"),
                        "[package]\nname = \"civsim-stone0\"\n",
                    )
                })
                .and_then(|_| fs::write(decoy_root.join("scripts/gates.toml"), ""))
                .and_then(|_| fs::write(decoy_root.join("scripts/gate_runner.py"), ""))
                .and_then(|_| fs::write(decoy_root.join(TOMBSTONE_REL), ""))
                .map_err(|error| format!("could not create repository-shaped decoy: {error}"))?;
            let canonical_decoy = fs::canonicalize(&decoy_root)
                .map_err(|error| format!("could not canonicalize repository decoy: {error}"))?;
            let expected_root = fs::canonicalize(&manifest_root)
                .map_err(|error| format!("could not canonicalize real repository root: {error}"))?;
            let decoy_has_identity = has_repository_root_shape(&canonical_decoy)
                && REPOSITORY_ROOT_MEMBERS.iter().all(|relative| {
                    validate_regular_repository_member(&canonical_decoy, relative).is_ok()
                });

            let original_path = std::env::var_os("PATH");
            std::env::set_var("PATH", &fake_bin);
            let explicit = canonicalize_and_validate_repository_root(&manifest_root);
            let discovered = discover_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")));
            let decoy_git_root = trusted_git_top_level(&canonical_decoy);
            let decoy_git_root_is_parent = decoy_git_root
                .as_ref()
                .is_ok_and(|observed| observed == &expected_root && observed != &canonical_decoy);
            let decoy_refused =
                canonicalize_and_validate_repository_root(&canonical_decoy).is_err();
            match original_path {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
            let explicit =
                explicit.map_err(|error| format!("explicit root binding failed: {error}"))?;
            let discovered =
                discovered.map_err(|error| format!("root discovery failed: {error}"))?;
            Ok((
                explicit,
                discovered,
                fake_git_sentinel.exists(),
                decoy_has_identity,
                decoy_git_root_is_parent,
                decoy_refused,
            ))
        })();
        match root_binding {
            Ok((
                explicit,
                discovered,
                fake_git_ran,
                decoy_has_identity,
                decoy_git_root_is_parent,
                decoy_refused,
            )) => {
                let expected =
                    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
                check(
                    "manifest-derived explicit root remains exact under a fake PATH Git",
                    expected
                        .as_ref()
                        .is_ok_and(|expected| expected == &explicit)
                        && explicit == discovered
                        && !fake_git_ran,
                );
                check(
                    "repository decoy satisfies the non-Git identity and file-shape checks",
                    decoy_has_identity,
                );
                check(
                    "trusted Git resolves the repository decoy to the distinct real parent root",
                    decoy_git_root_is_parent,
                );
                check(
                    "repository-shaped decoy is refused because it is not the trusted Git top level",
                    decoy_refused,
                );
            }
            Err(error) => check(
                &format!("repository-root binding fixture failed: {error}"),
                false,
            ),
        }
        check(
            "a nested crate directory is not accepted as an explicit repository root",
            canonicalize_and_validate_repository_root(Path::new(env!("CARGO_MANIFEST_DIR")))
                .is_err(),
        );
        let _ = fs::remove_dir_all(&root_binding_fixture);

        let python_boundary_root = std::env::temp_dir().join(format!(
            "civsim-stone0-python-boundary-self-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&python_boundary_root);
        let python_boundary = (|| -> Result<bool, String> {
            let scripts = python_boundary_root.join("scripts");
            let poison = python_boundary_root.join("poison");
            fs::create_dir_all(&scripts)
                .and_then(|_| fs::create_dir_all(&poison))
                .map_err(|error| format!("could not create Python-boundary fixture: {error}"))?;
            fs::write(
                scripts.join("pass.py"),
                "import json\n\
                 import os\n\
                 assert json.__name__ == \"json\"\n\
                 assert \"PYTHONPATH\" not in os.environ\n\
                 assert \"LD_PRELOAD\" not in os.environ\n",
            )
            .and_then(|_| fs::write(poison.join("json.py"), "raise RuntimeError('poisoned')\n"))
            .map_err(|error| format!("could not write Python-boundary fixture: {error}"))?;
            let poisoned_environment = [
                (
                    OsString::from("PYTHONPATH"),
                    poison.as_os_str().to_os_string(),
                ),
                (
                    OsString::from("LD_PRELOAD"),
                    poison.join("missing.so").into_os_string(),
                ),
            ];
            Ok(matches!(
                run_python_gate_with_environment(
                    &python_boundary_root,
                    "scripts/pass.py",
                    &[],
                    &poisoned_environment,
                ),
                ScriptResult::Clean
            ))
        })();
        match python_boundary {
            Ok(clean) => check(
                "Python import and dynamic-loader injection are removed before authority execution",
                clean,
            ),
            Err(error) => check(
                &format!("Python-boundary integration fixture failed: {error}"),
                false,
            ),
        }
        let _ = fs::remove_dir_all(&python_boundary_root);

        let shim_root = std::env::temp_dir().join(format!(
            "civsim-stone0-python-shim-self-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&shim_root);
        let shim_result = (|| -> Result<bool, String> {
            let scripts = shim_root.join("scripts");
            let bin = shim_root.join("bin");
            let sentinel = shim_root.join("shim-ran");
            fs::create_dir_all(&scripts)
                .and_then(|_| fs::create_dir_all(&bin))
                .map_err(|error| format!("could not create Python-shim fixture: {error}"))?;
            fs::write(scripts.join("pass.py"), "raise SystemExit(0)\n")
                .and_then(|_| {
                    fs::write(
                        bin.join("python3"),
                        "#!/bin/sh\n\
                         : > \"$CIVSIM_STONE0_SHIM_SENTINEL\"\n\
                         for argument in \"$@\"; do\n\
                           case \"$argument\" in\n\
                             civsim.stone0.python-authority-success.v1:*)\n\
                               printf '%s\\n' \"$argument\" >&2\n\
                               exit 0\n\
                               ;;\n\
                           esac\n\
                         done\n\
                         exit 0\n",
                    )
                })
                .map_err(|error| format!("could not write Python-shim fixture: {error}"))?;
            let mut permissions = fs::metadata(bin.join("python3"))
                .map_err(|error| format!("could not inspect Python shim: {error}"))?
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(bin.join("python3"), permissions)
                .map_err(|error| format!("could not arm Python shim: {error}"))?;
            let shim_environment = [
                (OsString::from("PATH"), bin.as_os_str().to_os_string()),
                (
                    OsString::from("CIVSIM_STONE0_SHIM_SENTINEL"),
                    sentinel.as_os_str().to_os_string(),
                ),
            ];
            let result = run_python_gate_with_environment(
                &shim_root,
                "scripts/pass.py",
                &[],
                &shim_environment,
            );
            Ok(matches!(result, ScriptResult::Clean) && !sentinel.exists())
        })();
        match shim_result {
            Ok(ignored) => check(
                "receipt-replaying PATH Python shim cannot enter the authority boundary",
                ignored,
            ),
            Err(error) => check(
                &format!("Python-shim integration fixture failed: {error}"),
                false,
            ),
        }
        let _ = fs::remove_dir_all(&shim_root);
    }
    #[cfg(not(unix))]
    check(
        "native platform without an interpreter trust root fails closed",
        trusted_python_interpreters().is_err(),
    );

    // password laundering: literal and base64 detection.
    let pw = "correct horse";
    let b64 = base64_standard(pw.as_bytes());
    let files = vec![
        RepoFile::new("clean.txt", b"nothing to see".to_vec()),
        RepoFile::new("literal.sh", format!("PASS={pw}\n").into_bytes()),
        RepoFile::new("encoded.txt", format!("token: {b64}").into_bytes()),
    ];
    let hits = find_password_hits(&files, pw);
    check(
        "password literal caught",
        hits.contains(&"literal.sh".to_string()),
    );
    check(
        "password base64 caught",
        hits.contains(&"encoded.txt".to_string()),
    );
    check(
        "password clean file not flagged",
        !hits.contains(&"clean.txt".to_string()),
    );
    check(
        "empty password matches nothing",
        find_password_hits(&files, "").is_empty(),
    );

    // tombstone detection, with the tombstone file itself excluded.
    let tombs = vec!["retired phrase alpha".to_string()];
    let tfiles = vec![
        RepoFile::new(
            "scripts/stone0_tombstones.txt",
            b"retired phrase alpha\n".to_vec(),
        ),
        RepoFile::new(
            "notes.md",
            b"someone wrote retired phrase alpha here".to_vec(),
        ),
    ];
    let thits = find_tombstone_hits(&tfiles, &tombs, &[TOMBSTONE_REL.to_string()]);
    check(
        "tombstone hit caught in other file",
        thits.iter().any(|(p, _)| p == "notes.md"),
    );
    check(
        "tombstone list itself not flagged",
        !thits
            .iter()
            .any(|(p, _)| p == "scripts/stone0_tombstones.txt"),
    );

    // override-env-name detection across shell, env, and yaml forms; prose is not flagged.
    let ofiles = vec![
        RepoFile::new("a.sh", b"export STONE0_OVERRIDE=hunter2\n".to_vec()),
        RepoFile::new("b.env", b"STONE0_OVERRIDE = value\n".to_vec()),
        RepoFile::new("c.yml", b"  STONE0_OVERRIDE: value\n".to_vec()),
        RepoFile::new(
            "prose.md",
            b"the STONE0_OVERRIDE variable is supplied out of band".to_vec(),
        ),
    ];
    let ohits = find_override_env_hits(&ofiles, &[]);
    check(
        "override-env shell caught",
        ohits.contains(&"a.sh".to_string()),
    );
    check(
        "override-env spaced caught",
        ohits.contains(&"b.env".to_string()),
    );
    check(
        "override-env yaml caught",
        ohits.contains(&"c.yml".to_string()),
    );
    check(
        "override-env prose not flagged",
        !ohits.contains(&"prose.md".to_string()),
    );
    let excluded = find_override_env_hits(&ofiles, &["a.sh".to_string()]);
    check(
        "override-env exclusion works",
        !excluded.contains(&"a.sh".to_string()),
    );

    // canary detection.
    let cfiles = vec![RepoFile::new(
        "leak.log",
        b"debug dump CANARY-XYZ-123 end".to_vec(),
    )];
    let chits = find_canary_hits(&cfiles, "CANARY-XYZ-123", &[]);
    check("canary caught", chits.contains(&"leak.log".to_string()));
    check(
        "empty canary matches nothing",
        find_canary_hits(&cfiles, "", &[]).is_empty(),
    );

    // secrets parsing: password is the first non-CANARY line, the canary is read separately.
    let (spw, scan) = parse_secrets("  my pass phrase  \nCANARY=abc123\n");
    check("secrets password parsed", spw == "my pass phrase");
    check("secrets canary parsed", scan.as_deref() == Some("abc123"));

    if problems.is_empty() {
        println!("stone0 gate self-test: PASS (every detector fires and none misfires)");
        0
    } else {
        println!("stone0 gate self-test: FAIL");
        for p in &problems {
            println!("  - {p}");
        }
        1
    }
}
